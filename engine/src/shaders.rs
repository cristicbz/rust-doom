use super::entities::{Entities, Entity, EntityId};
use super::errors::{ErrorKind, Result};
use super::platform;
use super::system::InfallibleSystem;
use super::window::Window;
use crate::internal_derive::DependenciesFrom;

use failchain::ResultExt;
use glium::program::{Program, ProgramCreationInput};
use idcontain::IdMapVec;
use log::{debug, error};
use math::{Mat4, Vec2, Vec3};
use std::fs::File;
use std::io::Read;
use std::io::Result as IoResult;
use std::num::{NonZeroU32, NonZeroU64};
use std::path::{Path, PathBuf};

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct ShaderId(pub EntityId);

pub struct ShaderConfig {
    pub root_path: PathBuf,
}

pub struct Shaders {
    map: IdMapVec<Entity, Shader>,
    root: PathBuf,
    global_bind_group_layout: wgpu::BindGroupLayout,
    material_bind_group_layout: wgpu::BindGroupLayout,
    model_bind_group_layout: wgpu::BindGroupLayout,
}

impl Shaders {
    pub fn add<VertexT: ShaderVertex>(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        asset_path: &'static str,
    ) -> Result<ShaderId> {
        let mut fragment_path = self.root.clone();
        fragment_path.push(asset_path);

        let wgsl_path = fragment_path.clone();
        fragment_path.set_extension("wgsl");

        let mut vertex_path = fragment_path.clone();
        fragment_path.set_extension("frag");
        vertex_path.set_extension("vert");

        let mut fragment_source = format!("#version {}\n", platform::GLSL_VERSION_STRING);
        let mut vertex_source = fragment_source.clone();

        debug!(
            "Loading shader {:?} (from {}, fragment={:?} and vert={:?})",
            name, asset_path, fragment_path, vertex_path
        );
        read_utf8_file(&fragment_path, &mut fragment_source)
            .chain_err(|| ErrorKind::ResourceIo("fragment shader", name))?;
        read_utf8_file(&vertex_path, &mut vertex_source)
            .chain_err(|| ErrorKind::ResourceIo("vertex shader", name))?;

        let program = Program::new(
            window.facade(),
            ProgramCreationInput::SourceCode {
                vertex_shader: &vertex_source,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                geometry_shader: None,
                fragment_shader: &fragment_source,
                transform_feedback_varyings: None,
                // TODO(cristicbz): More configurable things! SRGB should not be hard coded.
                outputs_srgb: true,
                uses_point_size: false,
            },
        )
        .map_err(ErrorKind::glium(name))?;

        let shader_module = window
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(wgsl_path.to_string_lossy().into()),
            });

        let pipeline_layout =
            window
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(name),
                    bind_group_layouts: &[
                        &self.global_bind_group_layout,
                        &self.material_bind_group_layout,
                        &self.model_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let pipeline = window
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "main_vs",
                    buffers: &[VertexT::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "main_fs",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: window.texture_format(),
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview: None,
            });

        debug!("Shader {:?} loaded successfully", name);
        let id = entities.add(parent, name)?;
        self.map.insert(id, Shader { program, pipeline });
        debug!("Added shader {:?} {:?} as child of {:?}.", name, id, parent);
        Ok(ShaderId(id))
    }

    pub fn get(&self, shader_id: ShaderId) -> Option<&Program> {
        self.map.get(shader_id.0).map(|shader| &shader.program)
    }

    pub(crate) fn get_pipeline(&self, shader_id: ShaderId) -> Option<&wgpu::RenderPipeline> {
        self.map
            .get(shader_id.0)
            .map(|shader: &Shader| &shader.pipeline)
    }

    pub(crate) fn global_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.global_bind_group_layout
    }

    pub(crate) fn material_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.material_bind_group_layout
    }

    pub(crate) fn model_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.model_bind_group_layout
    }
}

pub struct Shader {
    program: Program,
    pipeline: wgpu::RenderPipeline,
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    config: &'context ShaderConfig,
    entities: &'context Entities,
    window: &'context Window,
}

impl<'context> InfallibleSystem<'context> for Shaders {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "shaders"
    }

    fn create(deps: Dependencies) -> Self {
        let global_bind_group_layout = deps.window.device().create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("global_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(std::mem::size_of::<Mat4>() as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(std::mem::size_of::<u8>() as u64),
                        },
                        count: NonZeroU32::new(256),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(std::mem::size_of::<f32>() as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            },
        );
        let material_bind_group_layout = deps.window.device().create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("material_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                std::mem::size_of::<Vec2<f32>>() as u64
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(std::mem::size_of::<f32>() as u64),
                        },
                        count: None,
                    },
                ],
            },
        );
        let model_bind_group_layout =
            deps.window
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: NonZeroU64::new(
                                    std::mem::size_of::<Mat4>() as u64
                                ),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: NonZeroU64::new(
                                    std::mem::size_of::<Vec3<f32>>() as u64
                                ),
                            },
                            count: None,
                        },
                    ],
                });

        Shaders {
            map: IdMapVec::with_capacity(32),
            root: deps.config.root_path.clone(),
            global_bind_group_layout,
            material_bind_group_layout,
            model_bind_group_layout,
        }
    }

    fn update(&mut self, deps: Dependencies) {
        for &entity in deps.entities.last_removed() {
            if self.map.remove(entity).is_some() {
                debug!("Removed shader {:?}.", entity);
            }
        }
    }

    fn teardown(&mut self, deps: Dependencies) {
        self.update(deps);
    }

    fn destroy(mut self, deps: Dependencies) {
        self.update(deps);
        if !self.map.is_empty() {
            error!("Shaders leaked, {} instances.", self.map.len());
        }
    }
}

fn read_utf8_file(path: &Path, into: &mut String) -> IoResult<()> {
    File::open(path)?.read_to_string(into).map(|_| ())
}

pub trait ShaderVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

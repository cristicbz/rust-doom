use super::entities::{Entities, Entity, EntityId};
use super::errors::Result;
use super::system::InfallibleSystem;
use super::window::Window;
use crate::internal_derive::DependenciesFrom;
use crate::renderer::MSAA_SAMPLE_COUNT;

use idcontain::IdMapVec;
use log::{debug, error};
use math::{Mat4, Vec2, Vec3};
use std::num::NonZeroU64;
use std::path::PathBuf;

pub const LIGHTS_COUNT: usize = 256;

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct ShaderId(pub EntityId);

pub struct ShaderConfig {
    pub root_path: PathBuf,
}

pub struct Shaders {
    map: IdMapVec<Entity, Shader>,
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
        wgsl_source: &'static str,
    ) -> Result<ShaderId> {
        debug!("Loading shader {:?} (from {})", name, asset_path);

        let shader_module = window
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
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
                    count: MSAA_SAMPLE_COUNT,
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
        self.map.insert(id, Shader { pipeline });
        debug!("Added shader {:?} {:?} as child of {:?}.", name, id, parent);
        Ok(ShaderId(id))
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
    pipeline: wgpu::RenderPipeline,
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(
                                (LIGHTS_COUNT * std::mem::size_of::<u32>()) as u64,
                            ),
                        },
                        count: None,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                            binding: 1,
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

pub trait ShaderVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

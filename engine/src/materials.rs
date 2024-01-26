use super::entities::{Entities, Entity, EntityId};
use super::errors::Result;
use super::shaders::{ShaderId, Shaders};
use super::system::InfallibleSystem;
use super::uniforms::{UniformId, Uniforms};
use idcontain::IdMapVec;
use log::{debug, error};
use wgpu::util::DeviceExt;

pub const MAX_UNIFORMS: usize = 64;

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct MaterialId(pub EntityId);

pub struct Materials {
    map: IdMapVec<Entity, Material>,
}

impl Materials {
    pub fn update(&mut self, entities: &Entities) {
        for &entity in entities.last_removed() {
            if self.map.remove(entity).is_some() {
                debug!("Removed material {:?}.", entity);
            }
        }
    }

    pub fn add<'a>(
        &'a mut self,
        entities: &mut Entities,
        parent: EntityId,
        shader: ShaderId,
        name: &'static str,
    ) -> Result<MaterialRefMut<'a>> {
        let id = entities.add(parent, name)?;
        self.map.insert(
            id,
            Material {
                shader,
                uniforms: [None; MAX_UNIFORMS],
                bind_group: None,
            },
        );
        debug!(
            "Added material {:?} {:?} as child of {:?}.",
            name, id, shader
        );
        Ok(self
            .get_mut(MaterialId(id))
            .expect("missing just added material"))
    }

    pub fn get_mut(&mut self, id: MaterialId) -> Option<MaterialRefMut> {
        self.map
            .get_mut(id.0)
            .map(|material| MaterialRefMut { material, id })
    }

    pub fn get<'a>(
        &'a self,
        shaders: &'a Shaders,
        uniforms: &'a Uniforms,
        material_id: MaterialId,
    ) -> Option<MaterialRef<'a>> {
        let material = self.map.get(material_id.0)?;

        let Some(pipeline) = shaders.get_pipeline(material.shader) else {
            error!(
                "Missing pipeline {:?} for material {:?}",
                material.shader, material_id
            );
            return None;
        };

        let mut uniform_values = [None; MAX_UNIFORMS];
        for (value, &uniform) in (&mut uniform_values[..])
            .iter_mut()
            .zip(&material.uniforms[..])
        {
            if let Some((name, id)) = uniform {
                if let Some(uniform_value) = uniforms.get_value(id) {
                    *value = Some((name, uniform_value));
                } else {
                    error!(
                        "Missing uniform for material {:?}: name={:?} id={:?}",
                        material_id, name, id
                    );
                    return None;
                }
            } else {
                break;
            }
        }

        Some(MaterialRef {
            pipeline,
            bind_group: material
                .bind_group
                .as_ref()
                .expect("Bind group must be present when rendering"),
        })
    }
}

pub struct MaterialRefMut<'a> {
    material: &'a mut Material,
    id: MaterialId,
}

impl<'a> MaterialRefMut<'a> {
    pub fn add_uniform<I: Into<UniformId>>(&mut self, name: &'static str, id: I) -> &mut Self {
        let mut added = false;
        for uniform in &mut self.material.uniforms[..] {
            if uniform.is_none() {
                *uniform = Some((name, id.into()));
                added = true;
                break;
            }
        }
        if added {
            return self;
        }
        // TODO(cristicbz): Replace panic with error maybe? Or better solution for many uniforms.
        panic!(
            "Too many uniforms! material_id={:?} uniform={:?} max={}",
            self.id, name, MAX_UNIFORMS
        );
    }

    pub fn with_atlas(
        &mut self,
        atlas: &wgpu::Texture,
        device: &wgpu::Device,
        shaders: &Shaders,
    ) -> &mut Self {
        let atlas_view = atlas.create_view(&Default::default());
        let atlas_size = [atlas.width() as f32, atlas.height() as f32];
        let atlas_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&atlas_size),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        self.material.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &shaders.material_bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: atlas_size_buffer.as_entire_binding(),
                },
            ],
        }));
        self
    }

    pub fn with_sky_texture(
        &mut self,
        texture: &wgpu::Texture,
        tiled_band_size: f32,
        device: &wgpu::Device,
        shaders: &Shaders,
    ) -> &mut Self {
        let texture_view = texture.create_view(&Default::default());
        let tiled_band_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[tiled_band_size]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        self.material.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &shaders.material_bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: tiled_band_size_buffer.as_entire_binding(),
                },
            ],
        }));
        self
    }

    pub fn id(&self) -> MaterialId {
        self.id
    }
}

pub struct MaterialRef<'a> {
    pipeline: &'a wgpu::RenderPipeline,
    bind_group: &'a wgpu::BindGroup,
}

impl<'context> InfallibleSystem<'context> for Materials {
    type Dependencies = &'context Entities;

    fn debug_name() -> &'static str {
        "materials"
    }

    fn create(_deps: &Entities) -> Self {
        Materials {
            map: IdMapVec::with_capacity(32),
        }
    }

    fn update(&mut self, entities: &Entities) {
        for &entity in entities.last_removed() {
            if self.map.remove(entity).is_some() {
                debug!("Removed material {:?}.", entity);
            }
        }
    }

    fn teardown(&mut self, entities: &Entities) {
        self.update(entities);
    }

    fn destroy(mut self, entities: &Entities) {
        self.update(entities);
        if !self.map.is_empty() {
            error!("Materials leaked, {} instances.", self.map.len());
        }
    }
}

impl<'a> MaterialRef<'a> {
    pub(crate) fn pipeline(&self) -> &'a wgpu::RenderPipeline {
        &self.pipeline
    }

    pub(crate) fn bind_group(&self) -> &'a wgpu::BindGroup {
        &self.bind_group
    }
}

struct Material {
    shader: ShaderId,
    uniforms: [Option<(&'static str, UniformId)>; MAX_UNIFORMS],
    bind_group: Option<wgpu::BindGroup>,
}

use super::entities::{Entities, Entity, EntityId};
use super::errors::Result;
use super::shaders::{ShaderId, Shaders};
use super::system::InfallibleSystem;
use super::uniforms::{UniformId, Uniforms};
use glium::uniforms::{UniformValue, Uniforms as GliumUniforms};
use glium::Program;
use idcontain::IdMapVec;
use log::{debug, error};

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
        let material = if let Some(material) = self.map.get(material_id.0) {
            material
        } else {
            return None;
        };
        let shader = if let Some(shader) = shaders.get(material.shader) {
            shader
        } else {
            error!(
                "Missing shader {:?} for material {:?}",
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
            shader,
            uniform_values,
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

    pub fn id(&self) -> MaterialId {
        self.id
    }
}

pub struct MaterialRef<'a> {
    shader: &'a Program,
    uniform_values: [Option<(&'static str, UniformValue<'a>)>; MAX_UNIFORMS],
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
    pub fn shader(&self) -> &Program {
        self.shader
    }
}

impl<'material> GliumUniforms for MaterialRef<'material> {
    fn visit_values<'a, F>(&'a self, mut set_uniform: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        for uniform in &self.uniform_values[..] {
            if let Some((name, value)) = *uniform {
                set_uniform(name, value);
            } else {
                break;
            }
        }
    }
}

struct Material {
    shader: ShaderId,
    uniforms: [Option<(&'static str, UniformId)>; MAX_UNIFORMS],
}

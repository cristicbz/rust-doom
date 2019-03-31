use super::entities::{Entities, Entity, EntityId};
use super::materials::MaterialId;
use super::meshes::MeshId;
use super::system::InfallibleSystem;
use super::uniforms::{Mat4UniformId, Uniforms};
use crate::internal_derive::DependenciesFrom;
use idcontain::IdMapVec;
use log::{debug, error};
use math::prelude::*;
use math::Mat4;

impl RenderPipeline {
    pub fn modelview(&self) -> Mat4UniformId {
        self.modelview
    }

    pub fn projection(&self) -> Mat4UniformId {
        self.projection
    }

    pub fn set_camera(&mut self, camera: EntityId) {
        self.camera = Some(camera);
    }

    pub fn attach_model(&mut self, entity: EntityId, mesh: MeshId, material: MaterialId) {
        debug!(
            "Attaching model to entity {:?}: mesh={:?} material={:?}",
            entity, mesh, material
        );
        if let Some(old) = self.models.insert(entity, Model { mesh, material }) {
            error!(
                "Entity {:?} already had a model attached (mesh={:?}, material={:?}), replacing.",
                entity, old.mesh, old.material,
            );
        }
        debug!("Attached model to entity {:?}.", entity);
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    entities: &'context mut Entities,
    uniforms: &'context mut Uniforms,
}

pub struct RenderPipeline {
    pub(crate) models: IdMapVec<Entity, Model>,
    pub(crate) modelview: Mat4UniformId,
    pub(crate) projection: Mat4UniformId,
    pub(crate) root: EntityId,

    pub(crate) camera: Option<EntityId>,
}

impl<'context> InfallibleSystem<'context> for RenderPipeline {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "render_pipeline"
    }

    fn create(deps: Dependencies) -> Self {
        let root = deps.entities.add_root("render_pipeline");
        let modelview = deps
            .uniforms
            .add_mat4(deps.entities, root, "modelview_uniform", Mat4::one())
            .unwrap();
        let projection = deps
            .uniforms
            .add_mat4(deps.entities, root, "projection_uniform", Mat4::one())
            .unwrap();
        RenderPipeline {
            models: IdMapVec::with_capacity(128),
            root,
            projection,
            modelview,
            camera: None,
        }
    }

    fn teardown(&mut self, deps: Dependencies) {
        deps.entities.remove(self.root);
        self.camera = None;
    }
}

#[derive(Debug)]
pub(crate) struct Model {
    pub(crate) mesh: MeshId,
    pub(crate) material: MaterialId,
}

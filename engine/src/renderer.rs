use super::entities::{Entities, EntityId, Entity};
use super::errors::{Result, Error, NeededBy};
use super::materials::{Materials, MaterialId};
use super::meshes::{Meshes, MeshId};
use super::projections::Projections;
use super::shaders::Shaders;
use super::system::System;
use super::text::TextRenderer;
use super::tick::Tick;
use super::transforms::Transforms;
use super::uniforms::{Uniforms, Mat4UniformId};
use super::window::Window;
use glium::{BackfaceCullingMode, Depth, DepthTest, DrawParameters, Surface};
use idcontain::IdMapVec;
use math::Mat4;


impl Renderer {
    pub fn modelview(&self) -> Mat4UniformId {
        self.modelview
    }

    pub fn projection(&self) -> Mat4UniformId {
        self.projection
    }

    pub fn attach_model(
        &mut self,
        entity: EntityId,
        mesh: MeshId,
        material: MaterialId,
    ) -> Result<()> {
        debug!(
            "Attaching model to entity {:?}: mesh={:?} material={:?}",
            entity,
            mesh,
            material
        );
        if let Some(old) = self.models.insert(entity, Model { mesh, material }) {
            error!(
                "Entity {:?} already had a model attached (mesh={:?}, material={:?}), replacing.",
                entity,
                old.mesh,
                old.material,
                );
        }
        debug!("Attached model to entity {:?}.", entity);
        Ok(())
    }

    pub fn set_camera(&mut self, camera: EntityId) {
        self.camera = Some(camera);
    }
}

derive_dependencies_from! {
    pub struct Dependencies<'context> {
        entities: &'context mut Entities,
        meshes: &'context Meshes,
        materials: &'context Materials,
        shaders: &'context Shaders,
        text: &'context TextRenderer,
        window: &'context Window,
        transforms: &'context Transforms,
        projections: &'context Projections,
        uniforms: &'context mut Uniforms,
        tick: &'context Tick,
    }
}

pub struct Renderer {
    models: IdMapVec<Entity, Model>,
    draw_parameters: DrawParameters<'static>,

    entity: EntityId,
    modelview: Mat4UniformId,
    projection: Mat4UniformId,

    camera: Option<EntityId>,

    removed: Vec<usize>,
}


impl<'context> System<'context> for Renderer {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "renderer"
    }

    fn create(deps: Dependencies) -> Result<Self> {
        let entity = deps.entities.add_root("renderer");

        let modelview = deps.uniforms.add_mat4(
            deps.entities,
            entity,
            "modelview_uniform",
            Mat4::new_identity(),
        )?;
        let projection = deps.uniforms.add_mat4(
            deps.entities,
            entity,
            "projection_uniform",
            Mat4::new_identity(),
        )?;

        Ok(Renderer {
            entity,
            modelview,
            projection,
            models: IdMapVec::with_capacity(32),
            draw_parameters: DrawParameters {
                depth: Depth {
                    test: DepthTest::IfLess,
                    write: true,
                    ..Depth::default()
                },
                backface_culling: BackfaceCullingMode::CullClockwise,
                ..DrawParameters::default()
            },
            camera: None,
            removed: Vec::with_capacity(32),
        })
    }

    fn update(&mut self, deps: Dependencies) -> Result<()> {
        // If the current tick isn't a frame, skip all rendering.
        if !deps.tick.is_frame() {
            return Ok(());
        }

        // If no camera is given, skip rendering.
        let camera_id = if let Some(camera_id) = self.camera {
            camera_id
        } else {
            return Ok(());
        };

        // Compute view transform by inverting the camera entity transform.
        let view_transform = if let Some(transform) = deps.transforms.get_absolute(camera_id) {
            transform.inverse()
        } else {
            info!("Camera transform missing, disabling renderer.");
            self.camera = None;
            return Ok(());
        };
        let view_matrix = Mat4::from(&view_transform);

        // Set projection.
        *deps.uniforms.get_mat4_mut(self.projection).expect(
            "projection uniform missing",
        ) = *deps.projections.get_matrix(camera_id).expect(
            "camera projection missing",
        );

        // Render all the models in turn.
        let mut frame = deps.window.draw();
        for (index, &Model { mesh, material }) in self.models.access().iter().enumerate() {
            // For each model we need to assemble three things to render it: transform, mesh and
            // material. We get the entity id and query the corresponding systems for it.
            let entity = self.models.index_to_id(index).expect(
                "bad index enumerating models: mesh",
            );

            // If the mesh is missing, the entity was (probably) removed. So we add it to the
            // removed stack and continue.
            let mesh = if let Some(mesh) = deps.meshes.get(mesh) {
                mesh
            } else {
                info!(
                    "Mesh missing {:?} in model for entity {:?}, removing.",
                    mesh,
                    entity
                );
                self.removed.push(index);
                continue;
            };

            // If the model has a transform, then multiply it with the view transform to get the
            // modelview matrix. If there is no transform, model is assumed to be in world space, so
            // modelview = view.
            *deps.uniforms.get_mat4_mut(self.modelview).expect(
                "modelview uniform missing",
            ) = if let Some(model_transform) = deps.transforms.get_absolute(entity) {
                Mat4::from(view_transform.then(&model_transform))
            } else {
                view_matrix
            };

            let material =
                if let Some(material) = deps.materials.get(deps.shaders, deps.uniforms, material) {
                    material
                } else {
                    // If there is a mesh but no material, the model is badly set up. This is an
                    // error.
                    error!(
                        "Material missing {:?} in model for entity {:?}, removing.",
                        material,
                        entity
                    );
                    self.removed.push(index);
                    continue;
                };

            frame
                .draw(
                    &mesh,
                    &mesh,
                    material.shader(),
                    &material,
                    &self.draw_parameters,
                )
                .needed_by("renderer")?;
        }

        // Render text. TODO(cristicbz): text should render itself :(
        deps.text.render(&mut frame)?;

        // TODO(cristicbz): Re-architect a little bit to support rebuilding the context.
        frame.finish().expect(
            "Cannot handle context loss currently :(",
        );

        // Remove any missing models.
        for &index in self.removed.iter().rev() {
            self.models.remove_by_index(index);
        }
        self.removed.clear();
        Ok(())
    }

    fn teardown(&mut self, deps: Dependencies) -> Result<()> {
        let _ = deps.entities.remove(self.entity);
        self.camera = None;
        Ok(())
    }
}


#[derive(Debug)]
struct Model {
    mesh: MeshId,
    material: MaterialId,
}

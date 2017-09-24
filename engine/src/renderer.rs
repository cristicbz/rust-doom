use super::entities::{Entities, EntityId, Entity};
use super::errors::{Result, Error, NeededBy};
use super::materials::{Materials, MaterialId};
use super::meshes::{Meshes, MeshId};
use super::shaders::Shaders;
use super::system::System;
use super::text::TextRenderer;
use super::uniforms::Uniforms;
use super::window::Window;
use glium::{BackfaceCullingMode, Depth, DepthTest, DrawParameters, Surface};
use idcontain::IdMapVec;


pub struct Renderer {
    models: IdMapVec<Entity, Model>,
    draw_parameters: DrawParameters<'static>,
}

impl Renderer {
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
}

derive_dependencies_from! {
    pub struct Dependencies<'context> {
        entities: &'context Entities,
        meshes: &'context Meshes,
        materials: &'context Materials,
        shaders: &'context Shaders,
        text: &'context TextRenderer,
        uniforms: &'context Uniforms,
        window: &'context Window,
    }
}

impl<'context> System<'context> for Renderer {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "renderer"
    }

    fn create(_deps: Dependencies) -> Result<Self> {
        Ok(Renderer {
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
        })
    }

    fn update(&mut self, dependencies: Dependencies) -> Result<()> {
        let Dependencies {
            entities,
            meshes,
            materials,
            shaders,
            text,
            uniforms,
            window,
        } = dependencies;

        let mut frame = window.draw();

        for &entity in entities.last_removed() {
            if self.models.remove(entity).is_some() {
                debug!("Removed model {:?}.", entity);
            }
        }
        let mut errored = Vec::new();
        for (index, &Model { mesh, material }) in self.models.access().iter().enumerate() {
            let mesh = if let Some(mesh) = meshes.get(mesh) {
                mesh
            } else {
                let entity = self.models.index_to_id(index).expect(
                    "bad index enumerating models: mesh",
                );
                error!(
                    "Mesh missing {:?} in model for entity {:?}, removing.",
                    mesh,
                    entity
                );
                errored.push(entity);
                continue;
            };

            let material = if let Some(material) = materials.get(shaders, uniforms, material) {
                material
            } else {
                let entity = self.models.index_to_id(index).expect(
                    "bad index enumerating models: material",
                );
                error!(
                    "Material missing {:?} in model for entity {:?}, removing.",
                    material,
                    entity
                );
                errored.push(entity);
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

        for &entity in &errored {
            self.models.remove(entity);
        }

        text.render(&mut frame)?;

        // TODO(cristicbz): Re-architect a little bit to support rebuilding the context.
        frame.finish().expect(
            "Cannot handle context loss currently :(",
        );
        Ok(())
    }
}


#[derive(Debug)]
struct Model {
    mesh: MeshId,
    material: MaterialId,
}

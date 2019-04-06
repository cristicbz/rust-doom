use super::errors::{Error, ErrorKind, Result};
use super::materials::Materials;
use super::meshes::Meshes;
use super::pipeline::{Model, RenderPipeline};
use super::projections::Projections;
use super::shaders::Shaders;
use super::system::System;
use super::text::TextRenderer;
use super::tick::Tick;
use super::transforms::Transforms;
use super::uniforms::Uniforms;
use super::window::Window;
use crate::internal_derive::DependenciesFrom;
use failchain::ResultExt;
use glium::{BackfaceCullingMode, Depth, DepthTest, DrawParameters, Surface};
use log::{error, info};
use math::prelude::*;
use math::Mat4;

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    pipe: &'context mut RenderPipeline,
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

pub struct Renderer {
    draw_parameters: DrawParameters<'static>,
    removed: Vec<usize>,
}

impl<'context> System<'context> for Renderer {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "renderer"
    }

    fn create(_deps: Dependencies) -> Result<Self> {
        Ok(Renderer {
            draw_parameters: DrawParameters {
                depth: Depth {
                    test: DepthTest::IfLess,
                    write: true,
                    ..Depth::default()
                },
                backface_culling: BackfaceCullingMode::CullClockwise,
                ..DrawParameters::default()
            },
            removed: Vec::with_capacity(32),
        })
    }

    fn update(&mut self, deps: Dependencies) -> Result<()> {
        // If the current tick isn't a frame, skip all rendering.
        if !deps.tick.is_frame() {
            return Ok(());
        }

        let pipe = deps.pipe;

        // If no camera is given, skip rendering.
        let camera_id = if let Some(camera_id) = pipe.camera {
            camera_id
        } else {
            return Ok(());
        };

        // Compute view transform by inverting the camera entity transform.
        let view_transform = if let Some(transform) = deps.transforms.get_absolute(camera_id) {
            transform
                .inverse_transform()
                .expect("singular view transform")
        } else {
            info!("Camera transform missing, disabling renderer.");
            pipe.camera = None;
            return Ok(());
        };
        let view_matrix = view_transform.into();

        // Set projection.
        *deps
            .uniforms
            .get_mat4_mut(pipe.projection)
            .expect("projection uniform missing") = *deps
            .projections
            .get_matrix(camera_id)
            .expect("camera projection missing");

        // Render all the models in turn.
        let mut frame = deps.window.draw();
        for (index, &Model { mesh, material }) in pipe.models.access().iter().enumerate() {
            // For each model we need to assemble three things to render it: transform, mesh and
            // material. We get the entity id and query the corresponding systems for it.
            let entity = pipe
                .models
                .index_to_id(index)
                .expect("bad index enumerating models: mesh");

            // If the mesh is missing, the entity was (probably) removed. So we add it to the
            // removed stack and continue.
            let mesh = if let Some(mesh) = deps.meshes.get(mesh) {
                mesh
            } else {
                info!(
                    "Mesh missing {:?} in model for entity {:?}, removing.",
                    mesh, entity
                );
                self.removed.push(index);
                continue;
            };

            // If the model has a transform, then multiply it with the view transform to get the
            // modelview matrix. If there is no transform, model is assumed to be in world space, so
            // modelview = view.
            *deps
                .uniforms
                .get_mat4_mut(pipe.modelview)
                .expect("modelview uniform missing") =
                if let Some(model_transform) = deps.transforms.get_absolute(entity) {
                    Mat4::from(view_transform.concat(model_transform))
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
                        material, entity
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
                .map_err(ErrorKind::glium("renderer"))?;
        }

        // Render text. TODO(cristicbz): text should render itself :(
        deps.text
            .render(&mut frame)
            .chain_err(|| ErrorKind::System("render bypass", TextRenderer::debug_name()))?;

        // TODO(cristicbz): Re-architect a little bit to support rebuilding the context.
        frame
            .finish()
            .expect("Cannot handle context loss currently :(");

        // Remove any missing models.
        for &index in self.removed.iter().rev() {
            pipe.models.remove_by_index(index);
        }
        self.removed.clear();
        Ok(())
    }
}

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
use log::{error, info};
use math::prelude::*;

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
    removed: Vec<usize>,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    global_bind_group: wgpu::BindGroup,
}

impl<'context> System<'context> for Renderer {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "renderer"
    }

    fn create(deps: Dependencies) -> Result<Self> {
        let texture = deps
            .window
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Intermediate attachment"),
                size: deps.window.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: deps.window.texture_format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
        let view = texture.create_view(&Default::default());
        let depth_texture = deps
            .window
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth atachment"),
                size: deps.window.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
        let depth_view = depth_texture.create_view(&Default::default());
        let global_bind_group =
            deps.window
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Global bind group"),
                    layout: deps.shaders.global_bind_group_layout(),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: todo!(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: todo!(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: todo!(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: todo!(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: todo!(),
                        },
                    ],
                });
        Ok(Renderer {
            removed: Vec::with_capacity(32),
            _texture: texture,
            view,
            _depth_texture: depth_texture,
            depth_view,
            global_bind_group,
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
        // let view_matrix: Mat4 = view_transform.into();

        // Set projection.
        // TODO: This needs to be put in a buffer and copied to the GPU.
        *deps
            .uniforms
            .get_mat4_mut(pipe.projection)
            .expect("projection uniform missing") = *deps
            .projections
            .get_matrix(camera_id)
            .expect("camera projection missing");

        // Render all the models in turn.
        let mut frame = deps.window.draw();
        let surface_texture = deps.window.surface_texture()?;
        let view = surface_texture.texture.create_view(&Default::default());
        let mut encoder = deps
            .window
            .device()
            .create_command_encoder(&Default::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_bind_group(0, &self.global_bind_group, &[]);
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
                // TODO: Writing to uniforms can only be done by submitting a write buffer command, and needs to be done in advance of rendering.
                // *deps
                //     .uniforms
                //     .get_mat4_mut(pipe.modelview)
                //     .expect("modelview uniform missing") =
                //     if let Some(model_transform) = deps.transforms.get_absolute(entity) {
                //         Mat4::from(view_transform.concat(model_transform))
                //     } else {
                //         view_matrix
                //     };

                let material = if let Some(material) =
                    deps.materials.get(deps.shaders, deps.uniforms, material)
                {
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

                let model_bind_group = todo!();

                render_pass.set_pipeline(material.pipeline());
                render_pass.set_bind_group(1, material.bind_group(), &[]);
                render_pass.set_bind_group(2, &model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer());
                render_pass.set_index_buffer(mesh.index_buffer(), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.index_count(), 0, 0..1);
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
        }
        deps.window.queue().submit([encoder.finish()]);
        surface_texture.present();
        Ok(())
    }
}

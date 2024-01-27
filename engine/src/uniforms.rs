use crate::Shaders;

use super::entities::{Entities, Entity, EntityId};
use super::errors::Result;
use super::system::InfallibleSystem;
use super::window::Window;
use crate::internal_derive::DependenciesFrom;
use bytemuck::Pod;
use cgmath::SquareMatrix;
use glium::buffer::Content as BufferContent;
use glium::texture::buffer_texture::{BufferTexture, BufferTextureType};
use glium::uniforms::{AsUniformValue, UniformValue};
use idcontain::IdMapVec;
use log::{debug, error};
use math::{Mat4, Vec2, Vec2f};
use std::marker::PhantomData;
use wgpu::util::DeviceExt;

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct Texture2dId(EntityId);

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct FloatUniformId(EntityId);

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct BufferTextureId<T>
where
    [T]: BufferContent,
{
    id: EntityId,

    _phantom: PhantomData<*const T>,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct Mat4UniformId(EntityId);

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct Vec2fUniformId(EntityId);

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub enum UniformId {
    Texture2d(Texture2dId),
    Float(FloatUniformId),
    BufferTextureU8(BufferTextureId<u8>),
    Mat4(Mat4UniformId),
    Vec2f(Vec2fUniformId),
}

pub struct Uniforms {
    // TODO(cristicbz): Textures should be their own resource!
    texture2ds: IdMapVec<Entity, Texture2d>,
    floats: IdMapVec<Entity, f32>,
    buffer_textures_u8: IdMapVec<Entity, BufferTexture<u8>>,
    mat4s: IdMapVec<Entity, Mat4>,
    vec2fs: IdMapVec<Entity, Vec2f>,
    global_bind_group: Option<wgpu::BindGroup>,
    projection_buffer: wgpu::Buffer,
    time_buffer: wgpu::Buffer,
}

impl Uniforms {
    pub fn update(&mut self, entities: &Entities) {
        // Explicitly destructure all fields since the for-loop needs to be changed when adding a
        // new map.
        let Uniforms {
            ref mut texture2ds,
            ref mut floats,
            ref mut buffer_textures_u8,
            ref mut mat4s,
            ref mut vec2fs,
            global_bind_group: _,
            projection_buffer: _,
            time_buffer: _,
        } = *self;
        for &entity in entities.last_removed() {
            if texture2ds.remove(entity).is_some() {
                debug!("Removed uniform<texture2d> {:?}.", entity);
            }
            if floats.remove(entity).is_some() {
                debug!("Removed uniform<float> {:?}.", entity);
            }
            if buffer_textures_u8.remove(entity).is_some() {
                debug!("Removed uniform<buffer_textures_u8> {:?}.", entity);
            }
            if mat4s.remove(entity).is_some() {
                debug!("Removed uniform<mat4> {:?}.", entity);
            }
            if vec2fs.remove(entity).is_some() {
                debug!("Removed uniform<vec2> {:?}.", entity);
            }
        }
    }

    pub fn add_float(
        &mut self,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        initial: f32,
    ) -> Result<FloatUniformId> {
        let id = entities.add(parent, name)?;
        self.floats.insert(id, initial);
        debug!(
            "Added float uniform {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(FloatUniformId(id))
    }

    pub fn get_float_mut(&mut self, id: FloatUniformId) -> Option<&mut f32> {
        self.floats.get_mut(id.0)
    }

    pub fn add_mat4(
        &mut self,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        initial: Mat4,
    ) -> Result<Mat4UniformId> {
        let id = entities.add(parent, name)?;
        self.mat4s.insert(id, initial);
        debug!(
            "Added mat4 uniform {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(Mat4UniformId(id))
    }

    pub fn get_mat4_mut(&mut self, id: Mat4UniformId) -> Option<&mut Mat4> {
        self.mat4s.get_mut(id.0)
    }

    pub fn add_vec2f(
        &mut self,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        initial: Vec2f,
    ) -> Result<Vec2fUniformId> {
        let id = entities.add(parent, name)?;
        self.vec2fs.insert(id, initial);
        debug!(
            "Added vec2f uniform {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(Vec2fUniformId(id))
    }

    pub fn get_vec2f_mut(&mut self, id: Vec2fUniformId) -> Option<&mut Vec2f> {
        self.vec2fs.get_mut(id.0)
    }

    pub fn add_texture_2d<'a, PixelT: Pod>(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        pixels: &'a [PixelT],
        size: Vec2<usize>,
        format: wgpu::TextureFormat,
    ) -> Result<wgpu::Texture> {
        debug!(
            "Creating texture {:?}: pixels={}, size={:?}, format={:?}",
            name,
            pixels.len(),
            size,
            format,
        );
        let texture = window.device().create_texture_with_data(
            window.queue(),
            &wgpu::TextureDescriptor {
                label: Some(name),
                size: wgpu::Extent3d {
                    width: size[0] as u32,
                    height: size[1] as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(pixels),
        );
        debug!("Texture {:?} created successfully", name);
        let id = entities.add(parent, name)?;
        debug!(
            "Added texture {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(texture)
    }

    pub fn get_texture_2d_mut(&mut self, texture_id: Texture2dId) -> Option<Texture2dRefMut> {
        self.texture2ds
            .get_mut(texture_id.0)
            .map(|texture| Texture2dRefMut {
                texture,
                texture_id,
            })
    }

    // TODO(cristicbz): Make u8 a type param.
    pub fn add_persistent_buffer(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        size: usize,
        texture_type: BufferTextureType,
    ) -> Result<wgpu::Buffer> {
        debug!(
            "Creating persistent buffer {:?}, size={:?}, type={:?}",
            name, size, texture_type
        );
        let buffer = window.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(name),
            size: size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        debug!("Buffer texture {:?} created successfully", name);
        let id = entities.add(parent, name)?;
        debug!(
            "Added persistent buffer {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(buffer)
    }

    pub fn map_buffer<F, T: Default + Clone + Pod>(
        &mut self,
        buffer: &wgpu::Buffer,
        writer: F,
        queue: &wgpu::Queue,
    ) where
        F: FnOnce(&mut [T]),
    {
        let mut data = vec![T::default(); buffer.size() as usize / std::mem::size_of::<T>()];
        writer(&mut data);
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&data));
    }

    #[inline]
    pub fn get_value(&self, id: UniformId) -> Option<UniformValue> {
        match id {
            UniformId::Texture2d(_) => unimplemented!(),
            UniformId::Float(id) => self
                .floats
                .get(id.0)
                .map(|&value| UniformValue::Float(value)),
            UniformId::Vec2f(id) => self
                .vec2fs
                .get(id.0)
                .map(|vec2| UniformValue::Vec2([vec2[0], vec2[1]])),
            UniformId::Mat4(id) => self.mat4s.get(id.0).map(|mat4| {
                UniformValue::Mat4([
                    [mat4[0][0], mat4[0][1], mat4[0][2], mat4[0][3]],
                    [mat4[1][0], mat4[1][1], mat4[1][2], mat4[1][3]],
                    [mat4[2][0], mat4[2][1], mat4[2][2], mat4[2][3]],
                    [mat4[3][0], mat4[3][1], mat4[3][2], mat4[3][3]],
                ])
            }),
            UniformId::BufferTextureU8(id) => self
                .buffer_textures_u8
                .get(id.id)
                .map(AsUniformValue::as_uniform_value),
        }
    }

    pub fn set_globals(
        &mut self,
        device: &wgpu::Device,
        shaders: &Shaders,
        lights_buffer: &wgpu::Buffer,
        palette: &wgpu::Texture,
    ) {
        // TODO: Create a second sampler for the palette, using clamp address mode.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let palette_view = palette.create_view(&Default::default());
        self.global_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global bind group"),
            layout: shaders.global_bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.projection_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.time_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&palette_view),
                },
            ],
        }))
    }

    pub(crate) fn global_bind_group(&self) -> &wgpu::BindGroup {
        self.global_bind_group
            .as_ref()
            .expect("Global bind group must be set to render")
    }

    pub(crate) fn update_projection(&self, projection: Mat4, queue: &wgpu::Queue) {
        let projection: [[f32; 4]; 4] = projection.into();
        queue.write_buffer(
            &self.projection_buffer,
            0,
            bytemuck::cast_slice(&projection),
        );
    }
}

pub struct Texture2dRefMut<'uniforms> {
    texture: &'uniforms mut Texture2d,
    texture_id: Texture2dId,
}

impl<'uniforms> Texture2dRefMut<'uniforms> {
    pub fn replace_pixels<'pixels, PixelT: Pod>(
        &mut self,
        window: &Window,
        pixels: &'pixels [PixelT],
        size: Vec2<usize>,
    ) -> Result<()> {
        debug!(
            "Replacing texture {:?}: pixels={}, size={:?}",
            self.texture_id,
            pixels.len(),
            size,
        );
        window.queue().write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture.texture,
                mip_level: 1,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(pixels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((size[0] * std::mem::size_of::<PixelT>()) as u32),
                rows_per_image: Some(size[1] as u32),
            },
            wgpu::Extent3d {
                width: size[0] as u32,
                height: size[1] as u32,
                depth_or_array_layers: 1,
            },
        );

        debug!("Replaced texture {:?} successfully.", self.texture_id,);
        Ok(())
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    window: &'context Window,
    entities: &'context Entities,
}

impl<'context> InfallibleSystem<'context> for Uniforms {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "uniforms"
    }

    fn create(deps: Dependencies<'context>) -> Self {
        let projection: [[f32; 4]; 4] = Mat4::identity().into();
        let projection_buffer =
            deps.window
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Projection buffer"),
                    contents: bytemuck::cast_slice(&projection),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let time_buffer =
            deps.window
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Time buffer"),
                    contents: bytemuck::cast_slice(&[0.0f32]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        Uniforms {
            texture2ds: IdMapVec::with_capacity(32),
            floats: IdMapVec::with_capacity(32),
            buffer_textures_u8: IdMapVec::with_capacity(32),
            mat4s: IdMapVec::with_capacity(32),
            vec2fs: IdMapVec::with_capacity(32),
            global_bind_group: None,
            projection_buffer,
            time_buffer,
        }
    }

    fn update(&mut self, deps: Dependencies<'context>) {
        // Explicitly destructure all fields since the for-loop needs to be changed when adding a
        // new map.
        let Uniforms {
            ref mut texture2ds,
            ref mut floats,
            ref mut buffer_textures_u8,
            ref mut mat4s,
            ref mut vec2fs,
            global_bind_group: _,
            projection_buffer: _,
            time_buffer: _,
        } = *self;
        for &entity in deps.entities.last_removed() {
            if texture2ds.remove(entity).is_some() {
                debug!("Removed uniform<texture2d> {:?}.", entity);
            }
            if floats.remove(entity).is_some() {
                debug!("Removed uniform<float> {:?}.", entity);
            }
            if buffer_textures_u8.remove(entity).is_some() {
                debug!("Removed uniform<buffer_textures<u8>> {:?}.", entity);
            }
            if mat4s.remove(entity).is_some() {
                debug!("Removed uniform<mat4> {:?}.", entity);
            }
            if vec2fs.remove(entity).is_some() {
                debug!("Removed uniform<vec2> {:?}.", entity);
            }
        }
    }

    fn teardown(&mut self, deps: Dependencies<'context>) {
        self.update(deps.entities);
    }

    fn destroy(mut self, deps: Dependencies<'context>) {
        self.update(deps.entities);
        if !self.texture2ds.is_empty() {
            error!(
                "Uniforms <texture2d> leaked, {} instances.",
                self.texture2ds.len()
            );
        }

        if !self.floats.is_empty() {
            error!("Uniforms <float> leaked, {} instances.", self.floats.len());
        }

        if !self.buffer_textures_u8.is_empty() {
            error!(
                "Uniforms <buffer_textures<u8>> leaked, {} instances.",
                self.buffer_textures_u8.len()
            );
        }

        if !self.mat4s.is_empty() {
            error!("Uniforms <mat4> leaked, {} instances.", self.mat4s.len());
        }

        if !self.vec2fs.is_empty() {
            error!("Uniforms <vec2> leaked, {} instances.", self.vec2fs.len());
        }
    }
}

struct Texture2d {
    texture: wgpu::Texture,
}

impl From<Texture2dId> for UniformId {
    fn from(other: Texture2dId) -> Self {
        UniformId::Texture2d(other)
    }
}

impl From<FloatUniformId> for UniformId {
    fn from(other: FloatUniformId) -> Self {
        UniformId::Float(other)
    }
}

impl From<BufferTextureId<u8>> for UniformId {
    fn from(other: BufferTextureId<u8>) -> Self {
        UniformId::BufferTextureU8(other)
    }
}

impl From<Mat4UniformId> for UniformId {
    fn from(other: Mat4UniformId) -> Self {
        UniformId::Mat4(other)
    }
}

impl From<Vec2fUniformId> for UniformId {
    fn from(other: Vec2fUniformId) -> Self {
        UniformId::Vec2f(other)
    }
}

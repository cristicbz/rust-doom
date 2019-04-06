use super::entities::{Entities, Entity, EntityId};
use super::errors::{ErrorKind, Result};
use super::system::InfallibleSystem;
use super::window::Window;
use failchain::bail;
use glium::buffer::Content as BufferContent;
use glium::texture::buffer_texture::{BufferTexture, BufferTextureType};
use glium::texture::{ClientFormat, PixelValue, RawImage2d, Texture2d as GliumTexture2d};
use glium::uniforms::{AsUniformValue, SamplerBehavior, UniformValue};
use idcontain::IdMapVec;
use log::{debug, error};
use math::{Mat4, Vec2, Vec2f};
use std::borrow::Cow;
use std::marker::PhantomData;

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

    pub fn add_texture_2d<'a, PixelT: PixelValue>(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        pixels: &'a [PixelT],
        size: Vec2<usize>,
        format: ClientFormat,
        sampler: Option<SamplerBehavior>,
    ) -> Result<Texture2dId> {
        debug!(
            "Creating texture {:?}: pixels={}, size={:?}, format={:?}, sampler={:?}",
            name,
            pixels.len(),
            size,
            format,
            sampler,
        );
        let gl = GliumTexture2d::new(
            window.facade(),
            RawImage2d {
                data: Cow::Borrowed(pixels),
                width: size[0] as u32,
                height: size[1] as u32,
                format,
            },
        )
        .map_err(ErrorKind::glium(name))?;
        debug!("Texture {:?} created successfully", name);
        let id = entities.add(parent, name)?;
        self.texture2ds.insert(id, Texture2d { gl, sampler });
        debug!(
            "Added texture {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(Texture2dId(id))
    }

    pub fn get_texture_2d_mut(&mut self, texture_id: Texture2dId) -> Option<Texture2dRefMut> {
        self.texture2ds
            .get_mut(texture_id.0)
            .map(|texture| Texture2dRefMut {
                texture_id,
                texture,
            })
    }

    // TODO(cristicbz): Make u8 a type param.
    pub fn add_persistent_buffer_texture_u8(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        size: usize,
        texture_type: BufferTextureType,
    ) -> Result<BufferTextureId<u8>> {
        debug!(
            "Creating persistent buffer_texture<u7> {:?}, size={:?}, type={:?}",
            name, size, texture_type
        );
        let texture = BufferTexture::empty_persistent(window.facade(), size, texture_type)
            .map_err(ErrorKind::glium(name))?;
        debug!("Buffer texture {:?} created successfully", name);
        let id = entities.add(parent, name)?;
        self.buffer_textures_u8.insert(id, texture);
        debug!(
            "Added persistent buffer_texture<u8> {:?} {:?} as child of {:?}.",
            name, id, parent
        );
        Ok(BufferTextureId {
            id,
            _phantom: PhantomData,
        })
    }

    pub fn map_buffer_texture_u8<F>(&mut self, id: BufferTextureId<u8>, writer: F)
    where
        F: FnOnce(&mut [u8]),
    {
        // TODO(cristicbz): Handle missing.
        if let Some(buffer) = self.buffer_textures_u8.get_mut(id.id) {
            writer(&mut *buffer.map());
        }
    }

    pub fn add_texture2d_size(
        &mut self,
        entities: &mut Entities,
        name: &'static str,
        texture: Texture2dId,
    ) -> Result<Vec2fUniformId> {
        let size = self.texture2ds.get(texture.0).map(|texture| {
            let texture = &texture.gl;
            Vec2::new(
                texture.get_width() as f32,
                texture.get_height().unwrap_or(1) as f32,
            )
        });
        let size = if let Some(size) = size {
            size
        } else {
            bail!(ErrorKind::NoSuchComponent {
                context: "adding size uniform for texture",
                needed_by: Some(name),
                id: texture.0.cast(),
            });
        };
        self.add_vec2f(entities, texture.0, name, size)
    }

    #[inline]
    pub fn get_value(&self, id: UniformId) -> Option<UniformValue> {
        match id {
            UniformId::Texture2d(id) => self
                .texture2ds
                .get(id.0)
                .map(|texture| UniformValue::Texture2d(&texture.gl, texture.sampler)),
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
}

pub struct Texture2dRefMut<'uniforms> {
    texture: &'uniforms mut Texture2d,
    texture_id: Texture2dId,
}

impl<'uniforms> Texture2dRefMut<'uniforms> {
    pub fn get_sampler_mut(&mut self) -> &mut Option<SamplerBehavior> {
        &mut self.texture.sampler
    }

    pub fn replace_pixels<'pixels, PixelT: PixelValue>(
        &mut self,
        window: &Window,
        pixels: &'pixels [PixelT],
        size: Vec2<usize>,
        format: ClientFormat,
        sampler: Option<SamplerBehavior>,
    ) -> Result<()> {
        debug!(
            "Replacing texture {:?}: pixels={}, size={:?}, format={:?}, sampler={:?}",
            self.texture_id,
            pixels.len(),
            size,
            format,
            sampler,
        );
        self.texture.gl = GliumTexture2d::new(
            window.facade(),
            RawImage2d {
                data: Cow::Borrowed(pixels),
                width: size[0] as u32,
                height: size[1] as u32,
                format,
            },
        )
        .map_err(ErrorKind::glium("texture2d.replace_pixels"))?;

        debug!("Replaced texture {:?} successfully.", self.texture_id,);
        Ok(())
    }
}

impl<'context> InfallibleSystem<'context> for Uniforms {
    type Dependencies = &'context Entities;

    fn debug_name() -> &'static str {
        "uniforms"
    }

    fn create(_deps: &Entities) -> Self {
        Uniforms {
            texture2ds: IdMapVec::with_capacity(32),
            floats: IdMapVec::with_capacity(32),
            buffer_textures_u8: IdMapVec::with_capacity(32),
            mat4s: IdMapVec::with_capacity(32),
            vec2fs: IdMapVec::with_capacity(32),
        }
    }

    fn update(&mut self, entities: &Entities) {
        // Explicitly destructure all fields since the for-loop needs to be changed when adding a
        // new map.
        let Uniforms {
            ref mut texture2ds,
            ref mut floats,
            ref mut buffer_textures_u8,
            ref mut mat4s,
            ref mut vec2fs,
        } = *self;
        for &entity in entities.last_removed() {
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

    fn teardown(&mut self, entities: &Entities) {
        self.update(entities);
    }

    fn destroy(mut self, entities: &Entities) {
        self.update(entities);
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
    gl: GliumTexture2d,
    sampler: Option<SamplerBehavior>,
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

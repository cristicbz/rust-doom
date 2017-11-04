#![cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]

#[cfg(test)]
extern crate env_logger;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate log;
#[macro_use]
extern crate idcontain;

extern crate math;
extern crate num;
extern crate rusttype;
extern crate unicode_normalization;

#[macro_use]
mod context_macros;

pub mod context;
pub mod system;
pub mod type_list;

mod tick;
mod entities;
mod errors;
mod frame_timers;
mod input;
mod materials;
mod meshes;
mod platform;
mod projections;
mod renderer;
mod shaders;
mod text;
mod transforms;
mod uniforms;
mod window;

pub use self::context::{Context, ContextBuilder, ControlFlow};
pub use self::entities::{Entities, EntityId, Entity};
pub use self::errors::{Error, ErrorKind, Result};
pub use self::frame_timers::{FrameTimerId, FrameTimers};
pub use self::input::{Input, Analog2d, Gesture, Scancode, MouseButton};
pub use self::materials::{Materials, MaterialRefMut, MaterialId};
pub use self::meshes::{Meshes, MeshId, Mesh};
pub use self::projections::{Projections, Projection};
pub use self::renderer::Renderer;
pub use self::shaders::{Shaders, ShaderId, ShaderConfig};
pub use self::system::{System, InfallibleSystem};
pub use self::text::{Text, TextId, TextRenderer};
pub use self::tick::{Tick, TickIndex, Config as TickConfig};
pub use self::transforms::Transforms;
pub use self::uniforms::{Uniforms, Texture2dId, FloatUniformId, Mat4UniformId, Vec2fUniformId,
                         BufferTextureId, UniformId};
pub use self::window::{Window, WindowConfig};
pub use glium::texture::{ClientFormat, PixelValue};
pub use glium::texture::buffer_texture::BufferTextureType;
pub use glium::uniforms::{SamplerBehavior, SamplerWrapFunction, MinifySamplerFilter,
                          MagnifySamplerFilter};

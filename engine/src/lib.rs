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

extern crate glium_typed_buffer_any as glium_typed;
extern crate math;
extern crate num_traits;
extern crate rusttype;
extern crate unicode_normalization;

#[macro_use]
mod context_macros;

pub mod context;
pub mod system;
pub mod type_list;

mod entities;
mod errors;
mod frame_timers;
mod input;
mod materials;
mod meshes;
mod pipeline;
mod platform;
mod projections;
mod renderer;
mod shaders;
mod text;
mod tick;
mod transforms;
mod uniforms;
mod window;

pub use self::context::{Context, ContextBuilder, ControlFlow};
pub use self::entities::{Entities, Entity, EntityId};
pub use self::errors::{Error, ErrorKind, Result};
pub use self::frame_timers::{FrameTimerId, FrameTimers};
pub use self::input::{Analog2d, Gesture, Input, MouseButton, Scancode};
pub use self::materials::{MaterialId, MaterialRefMut, Materials};
pub use self::meshes::{Mesh, MeshId, Meshes};
pub use self::pipeline::RenderPipeline;
pub use self::projections::{Projection, Projections};
pub use self::renderer::Renderer;
pub use self::shaders::{ShaderConfig, ShaderId, Shaders};
pub use self::system::{InfallibleSystem, System};
pub use self::text::{Text, TextId, TextRenderer};
pub use self::tick::{Config as TickConfig, Tick, TickIndex};
pub use self::transforms::Transforms;
pub use self::uniforms::{
    BufferTextureId, FloatUniformId, Mat4UniformId, Texture2dId, UniformId, Uniforms,
    Vec2fUniformId,
};
pub use self::window::{Window, WindowConfig};
pub use glium::texture::buffer_texture::BufferTextureType;
pub use glium::texture::{ClientFormat, PixelValue};
pub use glium::uniforms::{
    MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior, SamplerWrapFunction,
};

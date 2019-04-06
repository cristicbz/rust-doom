#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

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

mod internal_derive {
    pub use super::context::DependenciesFrom;
    pub use engine_derive::InternalDependenciesFrom as DependenciesFrom;
}

pub use context::DependenciesFrom;
pub use engine_derive::DependenciesFrom;

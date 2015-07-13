#[macro_use] extern crate log;

extern crate common;
extern crate math;
extern crate libc;
extern crate sdl2;

pub use render::Renderer;
pub use render::RenderStep;
pub use render::StepId;
pub use shader::Shader;
pub use shader::ShaderLoader;
pub use shader::Uniform;
pub use texture::Texture;
pub use vbo::BufferBuilder;
pub use vbo::VertexBuffer;
pub use error::{Result, Error};
pub use window::Window;

mod error;
mod gl;
mod platform;
mod render;
mod shader;
mod texture;
mod vbo;
mod window;

#![feature(slicing_syntax, std_misc, core, path, )]

#[macro_use]
extern crate gl;
extern crate math;
extern crate base;

extern crate libc;

#[macro_use]
extern crate log;

pub use render::Renderer;
pub use render::RenderStep;
pub use shader::Shader;
pub use shader::ShaderLoader;
pub use shader::Uniform;
pub use texture::Texture;
pub use vbo::BufferBuilder;
pub use vbo::VertexBuffer;

mod render;
mod shader;
mod texture;
mod vbo;

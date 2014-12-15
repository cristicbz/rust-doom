#![feature(globs, macro_rules, phase, slicing_syntax)]

#[phase(plugin, link)]
extern crate gl;
extern crate math;
extern crate base;

extern crate libc;
#[phase(plugin, link)]
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

#[macro_use] extern crate log;
#[macro_use] extern crate glium;

extern crate common;
extern crate math;
extern crate libc;
extern crate sdl2;
extern crate glium_sdl2;

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
pub use scene::{Scene, SceneBuilder};
pub use vertex::{StaticBuffer, StaticVertex, SpriteBuffer, SpriteVertex, SkyBuffer, SkyVertex};
pub use vertex::{FlatBufferBuilder, WallBufferBuilder, DecorBufferBuilder, SkyBufferBuilder};

mod error;
mod gl;
mod platform;
mod render;
mod scene;
mod shader;
mod texture;
mod vbo;
mod vertex;
mod window;


use math::Vec2f;

#[derive(Copy, Clone, Debug)]
pub struct Bounds {
    pub pos: Vec2f,
    pub size: Vec2f,
    pub num_frames: usize,
    pub row_height: usize,
}

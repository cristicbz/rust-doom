#[macro_use]
extern crate log;
#[macro_use]
extern crate glium;

extern crate common;
extern crate math;
extern crate libc;
extern crate sdl2;
extern crate glium_sdl2;

pub use error::{Result, Error};
pub use window::Window;
pub use scene::{Scene, SceneBuilder};
pub use vertex::{StaticBuffer, StaticVertex, SpriteBuffer, SpriteVertex, SkyBuffer, SkyVertex};
pub use vertex::{FlatBufferBuilder, WallBufferBuilder, DecorBufferBuilder, SkyBufferBuilder};

mod error;
mod platform;
mod scene;
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

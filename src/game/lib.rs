#[macro_use] extern crate log;
#[macro_use] extern crate gl;

extern crate common;
extern crate gfx;
extern crate math;
extern crate wad;

extern crate num;
extern crate sdl2;
extern crate time;

pub use game::{Game, GameConfig};
pub use level::Level;

pub mod camera;
pub mod ctrl;
pub mod player;
pub mod level;
pub mod cached;
pub mod lights;
pub mod game;


pub const SHADER_ROOT: &'static str = "src/shaders";

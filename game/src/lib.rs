#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;

extern crate num;
extern crate sdl2;
extern crate time;

extern crate wad;
extern crate math;
extern crate engine;

mod camera;
mod ctrl;
mod level;
mod lights;
mod player;
mod world;
mod game;
mod errors;

pub use errors::{Error, Result, ErrorKind};
pub use game::{Game, GameConfig};
pub use level::Level;

pub const SHADER_ROOT: &'static str = "assets/shaders";

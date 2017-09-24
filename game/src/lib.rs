#![cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;

extern crate num;
extern crate time;

extern crate wad;
extern crate math;

#[macro_use]
extern crate engine;

#[macro_use]
extern crate glium;

mod level;
mod lights;
mod player;
mod world;
mod game;
mod errors;
mod hud;
mod vertex;
mod wad_system;

pub use errors::{Error, Result, ErrorKind};
pub use game::{Game, GameConfig};
pub use level::Level;

pub const SHADER_ROOT: &'static str = "assets/shaders";

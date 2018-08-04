#![cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;

#[macro_use]
extern crate engine;

// TODO(cristicbz): This is only needed because of the lack of `macro_reexport`.
#[macro_use]
extern crate glium;

extern crate idcontain;
extern crate time;
extern crate vec_map;

extern crate math;
extern crate wad;

mod errors;
mod game;
mod game_shaders;
mod hud;
mod level;
mod lights;
mod player;
mod vertex;
mod wad_system;
mod world;

pub use errors::{Error, ErrorKind, Result};
pub use game::{Game, GameConfig};
pub use level::Level;

pub const SHADER_ROOT: &str = "assets/shaders";

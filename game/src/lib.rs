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
extern crate num;
extern crate time;
extern crate vec_map;

extern crate wad;
extern crate math;

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

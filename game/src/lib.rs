#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

#[macro_use]
extern crate error_chain;

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

pub use self::errors::{Error, ErrorKind, Result};
pub use self::game::{Game, GameConfig};
pub use self::level::Level;

pub const SHADER_ROOT: &str = "assets/shaders";

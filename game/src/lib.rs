#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

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

pub use self::errors::{Error, Result};
pub use self::game::{create, Game, GameConfig};
pub use self::level::Level;

pub const SHADER_ROOT: &str = "assets/shaders";

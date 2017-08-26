pub mod camera;
pub mod ctrl;
pub mod game;
pub mod level;
pub mod lights;
pub mod player;
pub mod world;

pub use self::game::{Game, GameConfig};
pub use self::level::Level;

pub const SHADER_ROOT: &'static str = "src/shaders";

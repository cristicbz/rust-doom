mod error;
mod platform;
mod scene;
mod vertex;
mod window;
mod text;

pub use self::error::{Error, Result};
pub use self::scene::{Scene, SceneBuilder};
pub use self::window::Window;
pub use self::vertex::{SkyBuffer, SkyVertex, SpriteBuffer, SpriteVertex, StaticBuffer};
pub use self::vertex::{StaticVertex, DecorBufferBuilder, FlatBufferBuilder, SkyBufferBuilder};
pub use self::vertex::WallBufferBuilder;
pub use self::text::{Text, TextId, TextRenderer};

use math::Vec2f;

#[derive(Copy, Clone, Debug)]
pub struct Bounds {
    pub pos: Vec2f,
    pub size: Vec2f,
    pub num_frames: usize,
    pub row_height: usize,
}

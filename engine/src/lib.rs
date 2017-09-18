#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

extern crate glium_sdl2;
extern crate idcontain;
extern crate num;
extern crate sdl2;
extern crate time;

extern crate math;

mod errors;
mod platform;
mod scene;
mod text;
mod vertex;
mod window;
mod camera;
mod input;
mod frame_timers;

pub use camera::Camera;
pub use errors::{Error, ErrorKind, Result};
pub use frame_timers::{FrameTimerId, FrameTimers};
pub use input::{Input, Analog2d, Gesture, Scancode, MouseButton};
pub use scene::{Scene, SceneBuilder};
pub use text::{Text, TextId, TextRenderer};
pub use vertex::{SkyBuffer, SkyVertex, SpriteBuffer, SpriteVertex, StaticBuffer, Bounds};
pub use vertex::{StaticVertex, DecorBufferBuilder, FlatBufferBuilder, SkyBufferBuilder};
pub use vertex::WallBufferBuilder;
pub use window::Window;
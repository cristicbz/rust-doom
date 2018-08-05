#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate bincode;
extern crate byteorder;
extern crate indexmap;
extern crate regex;
extern crate serde;
extern crate time;
extern crate toml;
extern crate vec_map;

extern crate math;

mod archive;
mod error;
mod image;
mod level;
mod light;
mod meta;
mod name;
mod visitor;

pub mod tex;
pub mod types;
pub mod util;

pub use archive::Archive;
pub use error::{Error, ErrorKind, Result};
pub use image::Image;
pub use level::Level;
pub use light::{LightEffect, LightEffectKind, LightInfo};
pub use meta::{MoveEffectDef, SkyMetadata, ThingMetadata, TriggerType, WadMetadata};
pub use name::WadName;
pub use tex::{OpaqueImage, TextureDirectory, TransparentImage};
pub use visitor::{
    Branch, Decor, LevelAnalysis, LevelVisitor, LevelWalker, Marker, MoveEffect, ObjectId, SkyPoly,
    SkyQuad, StaticPoly, StaticQuad, Trigger,
};

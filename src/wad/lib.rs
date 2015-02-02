#![feature(slicing_syntax, io, core, std_misc, hash, path, collections)]

extern crate base;
extern crate gfx;
#[macro_use]
extern crate gl;
extern crate math;

#[macro_use]
extern crate log;

#[macro_use]
extern crate regex;
extern crate "rustc-serialize" as rustc_serialize;
extern crate time;
extern crate toml;

pub use archive::Archive;
pub use image::Image;
pub use level::Level;
pub use meta::WadMetadata;
pub use meta::SkyMetadata;
pub use name::WadName;
pub use tex::TextureDirectory;

mod name;
mod archive;
mod level;
mod image;
pub mod types;
pub mod util;
pub mod tex;
pub mod meta;

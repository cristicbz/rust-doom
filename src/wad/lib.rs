#![feature(macro_rules)]
#![feature(phase)]
#![feature(slicing_syntax)]

extern crate base;
extern crate gfx;
#[phase(plugin, link)]
extern crate gl;
extern crate math;

#[phase(plugin, link)]
extern crate log;
#[phase(plugin, link)]
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

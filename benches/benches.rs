#![feature(test)]
#[macro_use]
extern crate log;

extern crate common;
extern crate env_logger;
extern crate game;
extern crate getopts;
extern crate gfx;
extern crate sdl2;
extern crate time;
extern crate wad;

use common::GeneralError;
use game::Level;
use game::SHADER_ROOT;
use gfx::SceneBuilder;
use gfx::Window;
use std::error::Error;
use std::path::PathBuf;
use wad::{Archive, TextureDirectory};

extern crate test;
use test::Bencher;

fn check_wad(wad_file: &str, metadata_file: &str) -> Result<(), Box<Error>> {
    let sdl = try!(sdl2::init().map_err(|e| GeneralError(e.0)));
    let win = try!(Window::new(&sdl, 128, 128));

    info!("Loading all levels...");
    let wad = try!(Archive::open(&wad_file, &metadata_file));
    let textures = try!(TextureDirectory::from_archive(&wad));
    for level_index in 0..wad.num_levels() {
        let mut scene = SceneBuilder::new(&win, PathBuf::from(SHADER_ROOT));
        if let Err(e) = Level::new(&wad, &textures, level_index, &mut scene) {
            error!("reading level {}: {}", level_index, e);
        }
        if let Err(e) = scene.build() {
            error!("building scene for level {}: {}", level_index, e);
        }
    }
    info!("Done loading all levels.");
    Ok(())
}

#[bench]
fn freedoom1(b: &mut Bencher) {
    b.iter(|| check_wad("freedoom/freedoom1.wad", "doom.toml"))
}

#[bench]
fn freedoom2(b: &mut Bencher) {
    b.iter(|| check_wad("freedoom/freedoom2.wad", "doom.toml"))
}

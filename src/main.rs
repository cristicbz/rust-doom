#[macro_use]
extern crate log;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate clap;

extern crate bincode;
extern crate byteorder;
extern crate env_logger;
extern crate glium_sdl2;
extern crate libc;
extern crate num;
extern crate ordermap;
extern crate regex;
extern crate sdl2;
extern crate serde;
extern crate slab;
extern crate time;
extern crate toml;
extern crate vec_map;

pub mod common;
pub mod game;
pub mod gfx;
pub mod math;
pub mod wad;

use clap::{App, Arg, AppSettings};
use common::GeneralError;
use game::{Game, GameConfig, Level};
use game::SHADER_ROOT;
use gfx::SceneBuilder;
use gfx::Window;
use std::borrow::Cow;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use wad::{Archive, TextureDirectory};

pub struct Resolution {
    width: u32,
    height: u32,
}

impl FromStr for Resolution {
    type Err = GeneralError;
    fn from_str(size_str: &str) -> Result<Self, GeneralError> {
        size_str
            .find('x')
            .and_then(|x_index| if x_index == 0 || x_index + 1 == size_str.len() {
                None
            } else {
                Some((&size_str[..x_index], &size_str[x_index + 1..]))
            })
            .map(|(width, height)| {
                (width.parse::<u32>(), height.parse::<u32>())
            })
            .and_then(|size| match size {
                (Ok(width), Ok(height)) => Some(Resolution { width, height }),
                _ => None,
            })
            .ok_or_else(|| {
                GeneralError("resolution format must be WIDTHxHEIGHT".into())
            })
    }
}

pub enum RunMode {
    DisplayHelp(String),
    Check {
        wad_file: PathBuf,
        metadata_file: PathBuf,
    },
    ListLevelNames {
        wad_file: PathBuf,
        metadata_file: PathBuf,
    },
    Play(GameConfig),
}

impl RunMode {
    pub fn from_args() -> Result<RunMode, Box<Error>> {
        let matches = App::new("Rust Doom")
            .version("0.0.8")
            .author("Cristi Cobzarenco <cristi.cobzarenco@gmail.com>")
            .about("A Doom Renderer/Level Viewer written in Rust.")
            .settings(&[AppSettings::ColoredHelp])
            .arg(
                Arg::with_name("iwad")
                    .long("iwad")
                    .short("i")
                    .help("initial WAD file to use")
                    .value_name("FILE")
                    .default_value("doom1.wad"),
            )
            .arg(
                Arg::with_name("metadata")
                    .long("metadata")
                    .short("m")
                    .help("path to TOML metadata file")
                    .value_name("FILE")
                    .default_value("doom.toml"),
            )
            .arg(
                Arg::with_name("resolution")
                    .long("resolution")
                    .short("r")
                    .help("size of the game window")
                    .value_name("WIDTHxHEIGHT")
                    .default_value("1280x720"),
            )
            .arg(
                Arg::with_name("level")
                    .long("level")
                    .short("l")
                    .help("the index of the level to render")
                    .value_name("N")
                    .default_value("0"),
            )
            .arg(
                Arg::with_name("fov")
                    .long("fov")
                    .short("f")
                    .help("horizontal field of view")
                    .value_name("DEGREES")
                    .default_value("65"),
            )
            .arg(Arg::with_name("check").long("check").help(
                "load metadata and all levels in WAD, then exit",
            ))
            .arg(Arg::with_name("list-levels").long("list-levels").help(
                "list the names and indices of all the leves in the WAD, then exit",
            ))
            .get_matches();

        let wad_file: PathBuf = value_t!(matches, "iwad", String)?.into();
        let metadata_file: PathBuf = value_t!(matches, "metadata", String)?.into();

        Ok(if matches.is_present("check") {
            RunMode::Check {
                wad_file,
                metadata_file,
            }
        } else if matches.is_present("list-levels") {
            RunMode::ListLevelNames {
                wad_file,
                metadata_file,
            }
        } else {
            let Resolution { width, height } = value_t!(matches, "resolution", Resolution)?;
            let fov = value_t!(matches, "fov", f32)?;
            let level_index = value_t!(matches, "level", usize)?;

            RunMode::Play(GameConfig {
                wad_file,
                metadata_file,
                fov,
                width,
                height,
                level_index,
            })
        })
    }
}

#[cfg(not(test))]
fn run() -> Result<(), Box<Error>> {
    env_logger::init()?;

    match RunMode::from_args()? {
        RunMode::ListLevelNames {
            wad_file,
            metadata_file,
        } => {
            let wad = Archive::open(&wad_file, &metadata_file)?;
            for i_level in 0..wad.num_levels() {
                println!("{:3} {:8}", i_level, wad.level_name(i_level));
            }
        }
        RunMode::Check {
            wad_file,
            metadata_file,
        } => {
            let sdl = sdl2::init().map_err(GeneralError)?;
            let win = Window::new(&sdl, 128, 128)?;

            info!("Loading all levels...");
            let t0 = time::precise_time_s();
            let wad = Archive::open(&wad_file, &metadata_file)?;
            let textures = TextureDirectory::from_archive(&wad)?;
            for level_index in 0..wad.num_levels() {
                let mut scene = SceneBuilder::new(&win, PathBuf::from(SHADER_ROOT));
                if let Err(e) = Level::new(&wad, &textures, level_index, &mut scene) {
                    error!("reading level {}: {}", level_index, e);
                }
                if let Err(e) = scene.build() {
                    error!("building scene for level {}: {}", level_index, e);
                }
            }
            info!(
                "Done loading all levels in {:.4}s. Shutting down...",
                time::precise_time_s() - t0
            );
        }
        RunMode::DisplayHelp(help) => {
            println!("{}", help);
        }
        RunMode::Play(config) => {
            Game::new(config)?.run()?;
            info!("Game main loop ended, shutting down.");
        }
    }
    Ok(())
}

#[cfg(not(test))]
fn main() {
    use std::io;
    use std::io::Write;
    use std::env;
    use std::path::Path;

    if let Err(error) = run() {
        let program = env::args().next().unwrap_or_default();
        let filename = Path::new(&program).file_name().map_or_else(
            || {
                Cow::Borrowed("<cannot determine filename>")
            },
            |n| n.to_string_lossy(),
        );
        writeln!(io::stderr(), "{}: {}", filename, error).expect("failed to  write to stderr");
    }
}

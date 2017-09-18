#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate time;

extern crate game;
extern crate engine;
extern crate wad;

use clap::{App, Arg, AppSettings};
use engine::{Window, SceneBuilder};
use errors::{Result, Error};
use game::{Game, GameConfig, Level};
use game::SHADER_ROOT;
use std::path::PathBuf;
use std::str::FromStr;
use wad::{Archive, TextureDirectory};

mod errors {
    error_chain! {
        foreign_links {
            Argument(::clap::Error);
        }
        errors {}
        links {
            Engine(::engine::Error, ::engine::ErrorKind);
            Game(::game::Error, ::game::ErrorKind);
            Wad(::wad::Error, ::wad::ErrorKind);
        }
    }
}

pub struct Resolution {
    width: u32,
    height: u32,
}

impl FromStr for Resolution {
    type Err = Error;
    fn from_str(size_str: &str) -> Result<Self> {
        let size_if_ok = size_str
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
            });

        if let Some(size) = size_if_ok {
            Ok(size)
        } else {
            bail!("resolution format must be WIDTHxHEIGHT");
        }
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
    pub fn from_args() -> Result<RunMode> {
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
                    .default_value("assets/meta/doom.toml"),
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
                version: env!("CARGO_PKG_VERSION")
            })
        })
    }
}

fn run() -> Result<()> {
    env_logger::init().expect("Failed to set up logging.");

    match RunMode::from_args()? {
        RunMode::ListLevelNames {
            wad_file,
            metadata_file,
        } => {
            let wad = Archive::open(&wad_file, &metadata_file)?;
            for i_level in 0..wad.num_levels() {
                println!("{:3} {:8}", i_level, wad.level_lump(i_level)?.name());
            }
        }
        RunMode::Check {
            wad_file,
            metadata_file,
        } => {
            let win = Window::new(128, 128, "checking levels...")?;

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

quick_main!(run);

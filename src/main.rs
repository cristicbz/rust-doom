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
use game::{Game, GameConfig, Level};
use game::SHADER_ROOT;
use getopts::Options;
use gfx::SceneBuilder;
use gfx::Window;
use std::borrow::Cow;
use std::error::Error;
use std::path::PathBuf;
use wad::{Archive, TextureDirectory};

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
    pub fn from_args(args: &[String]) -> Result<RunMode, Box<Error>> {
        let mut opts = Options::new();
        opts.optopt("i",
                    "iwad",
                    "initial WAD file to use [default='doom1.wad']",
                    "FILE");
        opts.optopt("m",
                    "metadata",
                    "path to TOML metadata file [default='doom.toml']",
                    "FILE");
        opts.optopt("r",
                    "resolution",
                    "the size of the game window [default=1280x720]",
                    "WIDTHxHEIGHT");
        opts.optopt("l",
                    "level",
                    "the index of the level to render [default=0]",
                    "N");
        opts.optopt("f",
                    "fov",
                    "horizontal field of view to please TotalBiscuit [default=65]",
                    "FOV");
        opts.optflag("",
                     "check",
                     "load metadata and all levels in WAD, then exit");
        opts.optflag("",
                     "list-levels",
                     "list the names and indices of all the levels in the WAD, then exit");
        opts.optflag("h", "help", "print this help message and exit");
        let matches = try!(opts.parse(&args[1..]).map_err(|e| GeneralError(e.to_string())));

        if matches.opt_present("h") {
            return Ok(RunMode::DisplayHelp(opts.usage("rs_doom 0.0.7: A Rust Doom I/II \
                                                       Renderer.")));
        }


        let wad = matches.opt_str("iwad").unwrap_or("doom1.wad".to_owned()).into();
        let metadata = matches.opt_str("metadata").unwrap_or("doom.toml".to_owned()).into();

        Ok(if matches.opt_present("check") {
            RunMode::Check {
                wad_file: wad,
                metadata_file: metadata,
            }
        } else if matches.opt_present("list-levels") {
            RunMode::ListLevelNames {
                wad_file: wad,
                metadata_file: metadata,
            }
        } else {
            let (width, height) = try!(parse_window_size(&matches.opt_str("resolution")
                                                                 .unwrap_or("1280x720"
                                                                                .to_owned())));
            let fov = try!(matches.opt_str("fov")
                                  .unwrap_or("64".to_owned())
                                  .parse::<f32>()
                                  .map_err(|_| GeneralError("invalid value for fov".into())));
            let level = try!(matches.opt_str("level")
                                    .unwrap_or("0".to_owned())
                                    .parse::<usize>()
                                    .map_err(|_| GeneralError("invalid value for fov".into())));

            RunMode::Play(GameConfig {
                wad_file: wad,
                metadata_file: metadata,
                fov: fov,
                width: width,
                height: height,
                level_index: level,
            })
        })
    }
}

fn parse_window_size(size_str: &str) -> Result<(u32, u32), GeneralError> {
    size_str.find('x')
            .and_then(|x_index| {
                if x_index == 0 || x_index + 1 == size_str.len() {
                    None
                } else {
                    Some((&size_str[..x_index], &size_str[x_index + 1..]))
                }
            })
            .map(|(width, height)| (width.parse::<u32>(), height.parse::<u32>()))
            .and_then(|size| {
                match size {
                    (Ok(w), Ok(h)) => Some((w, h)),
                    _ => None,
                }
            })
            .ok_or_else(|| GeneralError("invalid window size (WIDTHxHEIGHT)".into()))
}

#[cfg(not(test))]
fn run(args: &[String]) -> Result<(), Box<Error>> {
    try!(env_logger::init());

    match try!(RunMode::from_args(args)) {
        RunMode::ListLevelNames { wad_file, metadata_file } => {
            let wad = try!(Archive::open(&wad_file, &metadata_file));
            for i_level in 0..wad.num_levels() {
                println!("{:3} {:8}", i_level, wad.level_name(i_level));
            }
        }
        RunMode::Check { wad_file, metadata_file } => {
            let sdl = try!(sdl2::init().map_err(|e| GeneralError(e.0)));
            let win = try!(Window::new(&sdl, 128, 128));

            info!("Loading all levels...");
            let t0 = time::precise_time_s();
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
            info!("Done loading all levels in {:.4}s. Shutting down...",
                  time::precise_time_s() - t0);
        }
        RunMode::DisplayHelp(help) => {
            println!("{}", help);
        }
        RunMode::Play(config) => {
            try!(try!(Game::new(config)).run());
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

    let args = env::args().collect::<Vec<_>>();

    if let Err(error) = run(&args) {
        let filename = Path::new(&args[0])
                           .file_name()
                           .map(|n| n.to_string_lossy())
                           .unwrap_or(Cow::Borrowed("<cannot determine filename>"));
        writeln!(io::stderr(), "{}: {}", filename, error)
            .ok()
            .expect("failed to  write to stderr");
    }
}

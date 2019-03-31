#[macro_use]
extern crate error_chain;

use game::{self, Game, GameConfig};
use log::info;
use std::path::PathBuf;
use structopt::StructOpt;
use wad::Archive;

use self::errors::Result;

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

fn parse_resolution(size_str: &str) -> Result<(u32, u32)> {
    let size_if_ok = size_str
        .find('x')
        .and_then(|x_index| {
            if x_index == 0 || x_index + 1 == size_str.len() {
                None
            } else {
                Some((&size_str[..x_index], &size_str[x_index + 1..]))
            }
        })
        .map(|(width, height)| (width.parse::<u32>(), height.parse::<u32>()))
        .and_then(|size| match size {
            (Ok(width), Ok(height)) => Some((width, height)),
            _ => None,
        });

    if let Some(size) = size_if_ok {
        Ok(size)
    } else {
        bail!("resolution format must be WIDTHxHEIGHT");
    }
}

#[derive(StructOpt, Copy, Clone)]
pub enum Command {
    /// Load metadata and all levels in WAD, then exit.
    #[structopt(name = "check")]
    Check,

    /// List the names and indices of all the leves in the WAD, then exit.
    #[structopt(name = "list-levels")]
    ListLevelNames,
}

#[derive(StructOpt)]
#[structopt(
    name = "Rusty Doom",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Args {
    #[structopt(
        short = "i",
        long = "iwad",
        default_value = "doom1.wad",
        value_name = "FILE",
        parse(from_os_str)
    )]
    /// Initial WAD file to use.
    iwad: PathBuf,

    #[structopt(
        short = "m",
        long = "metadata",
        default_value = "assets/meta/doom.toml",
        value_name = "FILE",
        parse(from_os_str)
    )]
    /// Path to TOML metadata file.
    metadata: PathBuf,

    #[structopt(
        short = "r",
        long = "resolution",
        default_value = "1280x720",
        value_name = "WIDTHxHEIGHT",
        parse(try_from_str = "parse_resolution")
    )]
    /// Size of the game window.
    resolution: (u32, u32),

    #[structopt(
        short = "l",
        long = "level",
        default_value = "0",
        help = "the index of the level to render",
        value_name = "N"
    )]
    /// The index of the level to render (0-based).
    level_index: usize,

    #[structopt(
        short = "f",
        long = "fov",
        default_value = "65",
        value_name = "DEGREES"
    )]
    /// Horizontal field of view.
    fov: f32,

    #[structopt(subcommand)]
    command: Option<Command>,
}

impl Args {
    pub fn into_config(self) -> GameConfig {
        GameConfig {
            wad_file: self.iwad,
            metadata_file: self.metadata,
            fov: self.fov,
            width: self.resolution.0,
            height: self.resolution.1,
            version: env!("CARGO_PKG_VERSION"),
            initial_level_index: self.level_index,
        }
    }
}

fn run() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    )
    .default_format_timestamp(false)
    .init();

    let args = Args::from_args();
    match args.command {
        None => {
            let mut game = game::create(&args.into_config())?;
            game.run()?;
            info!("Game main loop ended, shutting down...");
        }
        Some(Command::Check) => {
            let mut game = game::create(&GameConfig {
                initial_level_index: 0,
                ..args.into_config()
            })?;
            info!("Loading all levels...");
            let t0 = time::precise_time_s();
            for level_index in 1..game.num_levels() {
                game.load_level(level_index)?;
            }
            info!(
                "Done loading all levels in {:.4}s. Shutting down...",
                time::precise_time_s() - t0
            );
        }
        Some(Command::ListLevelNames) => {
            let wad = Archive::open(&args.iwad, &args.metadata)?;
            for i_level in 0..wad.num_levels() {
                println!("{:3} {:8}", i_level, wad.level_lump(i_level)?.name());
            }
        }
    }
    info!("Clean shutdown.");
    Ok(())
}

quick_main!(run);

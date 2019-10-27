use failure::{bail, Error};
use game::{self, Game, GameConfig};
use log::{error, info};
use math::DurationExt;
use std::env;
use std::path::PathBuf;
use std::process;
use std::time::Instant;
use structopt::StructOpt;
use wad::Archive;

#[derive(StructOpt)]
#[structopt(
    name = "Rusty Doom",
    setting = structopt::clap::AppSettings::ColoredHelp
)]
struct App {
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
        parse(try_from_str = parse_resolution)
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

#[derive(StructOpt, Copy, Clone)]
enum Command {
    /// Load metadata and all levels in WAD, then exit.
    #[structopt(name = "check")]
    Check,

    /// List the names and indices of all the leves in the WAD, then exit.
    #[structopt(name = "list-levels")]
    ListLevelNames,
}

impl App {
    /// Parse options from command-line arguments and run.
    pub fn run_from_args() -> Result<(), Error> {
        Self::from_args().run()
    }

    /// Either play the game (if no `Command` was passed), or perform the command.
    pub fn run(self) -> Result<(), Error> {
        // Init logging, with default `info` level.
        env_logger::Builder::from_env(
            env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
        )
        .format_timestamp(None)
        .init();

        match self.command {
            None => {
                let mut game = game::create(&self.into_config())?;
                game.run()?;
                info!("Game main loop ended, shutting down...");
            }
            Some(Command::Check) => {
                let mut game = game::create(&GameConfig {
                    initial_level_index: 0,
                    ..self.into_config()
                })?;
                info!("Loading all levels...");
                let t0 = Instant::now();
                for level_index in 1..game.num_levels() {
                    game.load_level(level_index)?;
                }
                info!(
                    "Done loading all levels in {:.4}s. Shutting down...",
                    t0.elapsed().f64_seconds()
                );
            }
            Some(Command::ListLevelNames) => {
                let wad = Archive::open(&self.iwad, &self.metadata)?;
                for i_level in 0..wad.num_levels() {
                    println!("{:3} {:8}", i_level, wad.level_lump(i_level)?.name());
                }
            }
        }
        info!("Clean shutdown.");
        Ok(())
    }

    /// Populate `GameConfig` fields from the parsed command-line arguments.
    fn into_config(self) -> GameConfig {
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

/// Parse a resolution string like `WIDTHxHEIGHT` into `(width, height)`.
fn parse_resolution(size_str: &str) -> Result<(u32, u32), Error> {
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
        bail!("Resolution format must be WIDTHxHEIGHT");
    }
}

fn main() {
    if let Err(error) = App::run_from_args() {
        error!("Fatal error: {}", error);
        let mut cause = error.as_fail();
        while let Some(new_cause) = cause.cause() {
            cause = new_cause;
            error!("    caused by: {}", cause);
        }
        if env::var("RUST_BACKTRACE")
            .map(|value| value == "1")
            .unwrap_or(false)
        {
            error!("Backtrace:\n{}", error.backtrace());
        } else {
            error!("Run with RUST_BACKTRACE=1 to capture backtrace.");
        }
        process::exit(1);
    }
}

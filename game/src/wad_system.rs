use super::errors::{Result, Error};
use engine::System;
use std::path::PathBuf;
use wad::{Archive, TextureDirectory};

pub struct Config {
    pub wad_path: PathBuf,
    pub metadata_path: PathBuf,
}

pub struct WadSystem {
    pub archive: Archive,
    pub textures: TextureDirectory,
}


impl<'context> System<'context> for WadSystem {
    type Dependencies = &'context Config;
    type Error = Error;

    fn debug_name() -> &'static str {
        "wad"
    }

    fn create(config: &Config) -> Result<Self> {
        let archive = Archive::open(&config.wad_path, &config.metadata_path)?;
        let textures = TextureDirectory::from_archive(&archive)?;
        Ok(WadSystem { archive, textures })
    }
}

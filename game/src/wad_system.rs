use super::errors::{Error, ErrorKind, Result};
use engine::{DependenciesFrom, System};
use failchain::{bail, ResultExt};
use log::info;
use std::path::PathBuf;
use wad::{
    Archive, Level as WadLevel, LevelAnalysis, LevelVisitor, LevelWalker, Result as WadResult,
    TextureDirectory, WadName,
};

#[derive(Debug)]
pub struct Config {
    pub wad_path: PathBuf,
    pub metadata_path: PathBuf,
    pub initial_level_index: usize,
}

pub struct WadSystem {
    pub archive: Archive,
    pub textures: TextureDirectory,
    pub level: WadLevel,
    pub analysis: LevelAnalysis,

    level_name: WadName,
    current_level_index: usize,
    next_level_index: usize,
    level_changed: bool,
}

impl WadSystem {
    pub fn level_name(&self) -> WadName {
        self.level_name
    }

    pub fn level_index(&self) -> usize {
        self.current_level_index
    }

    pub fn change_level(&mut self, new_level_index: usize) {
        self.next_level_index = new_level_index;
    }

    pub fn level_changed(&self) -> bool {
        self.level_changed
    }

    pub fn walk<V: LevelVisitor>(&self, visitor: &mut V) {
        LevelWalker::new(
            &self.level,
            &self.analysis,
            &self.textures,
            self.archive.metadata(),
            visitor,
        )
        .walk();
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    config: &'context Config,
}

impl<'context> System<'context> for WadSystem {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "wad"
    }

    fn create(deps: Dependencies) -> Result<Self> {
        let (archive, textures, level_index, level_name) = (|| -> WadResult<_> {
            let archive = Archive::open(&deps.config.wad_path, &deps.config.metadata_path)?;
            let textures = TextureDirectory::from_archive(&archive)?;
            let level_index = deps.config.initial_level_index;
            let level_name = archive.level_lump(level_index)?.name();
            Ok((archive, textures, level_index, level_name))
        })()
        .chain_err(|| ErrorKind(format!("WAD setup failed with: {:#?}", deps.config)))?;

        if level_index >= archive.num_levels() {
            bail!(
                ErrorKind,
                "Level index {} is not in valid range 0..{}, see --list-levels for level names.",
                level_index,
                archive.num_levels()
            );
        }

        info!(
            "Loading initial level {:?} ({})...",
            level_name, level_index
        );
        let level = WadLevel::from_archive(&archive, level_index).chain_err(|| {
            ErrorKind(format!(
                "when loading WAD level with config {:#?}",
                deps.config
            ))
        })?;
        info!("Analysing level...");
        let analysis = LevelAnalysis::new(&level, archive.metadata());

        Ok(WadSystem {
            archive,
            textures,
            level,
            analysis,
            current_level_index: level_index,
            next_level_index: level_index,
            level_changed: false,
            level_name,
        })
    }

    fn update(&mut self, _deps: Dependencies) -> Result<()> {
        self.level_changed = false;

        if self.next_level_index != self.current_level_index {
            if self.next_level_index >= self.archive.num_levels() {
                info!(
                    "New level index {} is out of bounds, keeping current.",
                    self.next_level_index
                );
                self.next_level_index = self.current_level_index;
            } else {
                self.current_level_index = self.next_level_index;
                self.level_name = self
                    .archive
                    .level_lump(self.next_level_index)
                    .chain_err(|| {
                        ErrorKind(format!(
                            "while accessing level name for next level request {}",
                            self.next_level_index
                        ))
                    })?
                    .name();
                info!(
                    "Loading new level {:?} ({})...",
                    self.level_name, self.next_level_index
                );
                self.level = WadLevel::from_archive(&self.archive, self.current_level_index)
                    .chain_err(|| {
                        ErrorKind(format!(
                            "while loading next level {} ({}) for next level request",
                            self.level_name, self.next_level_index
                        ))
                    })?;
                info!("Analysing new level...");
                self.analysis = LevelAnalysis::new(&self.level, self.archive.metadata());
                info!("Level replaced.");
                self.level_changed = true;
            }
        }
        Ok(())
    }
}

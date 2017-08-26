use super::error::{InFile, Result};
use super::name::WadName;
use super::types::ThingType;
use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

#[derive(Debug, Serialize, Deserialize)]
pub struct SkyMetadata {
    pub texture_name: WadName,
    pub level_pattern: String,
    pub tiled_band_size: f32,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct AnimationMetadata {
    pub flats: Vec<Vec<WadName>>,
    pub walls: Vec<Vec<WadName>>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ThingMetadata {
    pub thing_type: ThingType,
    pub sprite: String,
    pub sequence: String,
    pub hanging: bool,
    pub radius: u32,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ThingDirectoryMetadata {
    pub decorations: Vec<ThingMetadata>,
    pub weapons: Vec<ThingMetadata>,
    pub powerups: Vec<ThingMetadata>,
    pub artifacts: Vec<ThingMetadata>,
    pub ammo: Vec<ThingMetadata>,
    pub keys: Vec<ThingMetadata>,
    pub monsters: Vec<ThingMetadata>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct WadMetadata {
    pub sky: Vec<SkyMetadata>,
    pub animations: AnimationMetadata,
    pub things: ThingDirectoryMetadata,
}
impl WadMetadata {
    pub fn from_file<P: AsRef<Path>>(path: &P) -> Result<WadMetadata> {
        let mut contents = String::new();
        let path = path.as_ref();
        File::open(path)?.read_to_string(&mut contents)?;
        WadMetadata::from_text(&contents).in_file(path)
    }

    pub fn from_text(text: &str) -> Result<WadMetadata> {
        Ok(toml::from_str(text)?)
    }

    pub fn sky_for(&self, name: &WadName) -> Option<&SkyMetadata> {
        self.sky
            .iter()
            .find(|sky| {
                Regex::new(&sky.level_pattern)
                    .map(|r| r.is_match(name.as_ref()))
                    .unwrap_or_else(|_| {
                        warn!(
                            "Invalid level pattern {} for sky {}.",
                            sky.level_pattern,
                            sky.texture_name
                        );
                        false
                    })
            })
            .or_else(|| if let Some(sky) = self.sky.get(0) {
                warn!(
                    "No sky found for level {}, using {}.",
                    name,
                    sky.texture_name
                );
                Some(sky)
            } else {
                error!("No sky metadata provided.");
                None
            })
    }

    pub fn find_thing(&self, thing_type: ThingType) -> Option<&ThingMetadata> {
        self.things
            .decorations
            .iter()
            .find(|t| t.thing_type == thing_type)
            .or_else(|| {
                self.things.weapons.iter().find(
                    |t| t.thing_type == thing_type,
                )
            })
            .or_else(|| {
                self.things.powerups.iter().find(
                    |t| t.thing_type == thing_type,
                )
            })
            .or_else(|| {
                self.things.artifacts.iter().find(
                    |t| t.thing_type == thing_type,
                )
            })
            .or_else(|| {
                self.things.ammo.iter().find(|t| t.thing_type == thing_type)
            })
            .or_else(|| {
                self.things.keys.iter().find(|t| t.thing_type == thing_type)
            })
            .or_else(|| {
                self.things.monsters.iter().find(
                    |t| t.thing_type == thing_type,
                )
            })
    }
}

#[cfg(test)]
mod test {
    use super::WadMetadata;

    #[test]
    fn test_wad_metadata() {
        WadMetadata::from_text(
            r#"
            [[sky]]
                level_pattern = "MAP(0[1-9]|10|11)"
                texture_name = "SKY1"
                tiled_band_size = 0.15
            [[sky]]
                level_pattern = "MAP(1[2-9]|20)"
                texture_name = "SKY2"
                tiled_band_size = 0.15
            [[sky]]
                level_pattern = "MAP(2[1-9]|32)"
                texture_name = "SKY3"
                tiled_band_size = 0.15
            [animations]
                flats = [
                    ["NUKAGE1", "NUKAGE2", "NUKAGE3"],
                    [],
                ]
                walls = [
                    [],
                    ["DBRAIN1", "DBRAIN2", "DBRAIN3",  "DBRAIN4"],
                ]
            [things]
                [[things.decorations]]
                    thing_type = 10
                    radius = 16
                    sprite = "PLAY"
                    sequence = "W"
                    obstacle = false
                    hanging = false

                [[things.decorations]]
                    thing_type = 12
                    radius = 8
                    sprite = "PLAY"
                    sequence = "W"
                    obstacle = false
                    hanging = false

                [[things.weapons]]
                    # BFG 9000
                    thing_type = 2006
                    radius = 20
                    sprite = "BFUG"
                    sequence = "A"
                    hanging = false

                [[things.artifacts]]
                    # Computer map
                    thing_type = 2026
                    radius = 20
                    sprite = "PMAP"
                    sequence = "ABCDCB"
                    hanging = false

                [[things.ammo]]
                    # Box of ammo
                    thing_type = 2048
                    radius = 20
                    sprite = "AMMO"
                    sequence = "A"
                    hanging = false

                [[things.powerups]]
                    # Backpack
                    thing_type = 8
                    radius = 20
                    sprite = "BPAK"
                    sequence = "A"
                    hanging = false

                [[things.keys]]
                    # Red keycard
                    thing_type = 13
                    radius = 20
                    sprite = "RKEY"
                    sequence = "AB"
                    hanging = false

                [[things.monsters]]
                    # Baron of Hell
                    thing_type = 3003
                    radius = 24
                    sprite = "BOSS"
                    sequence = "A"
                    hanging = false
        "#,
        ).expect("test: could not parse test metadata");
    }
}

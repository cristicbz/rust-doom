use super::errors::{ErrorKind, Result};
use super::name::WadName;
use super::types::{SpecialType, ThingType, WadCoord};
use failchain::ResultExt;
use indexmap::IndexMap;
use log::{error, warn};
use regex::Regex;
use serde::{de::Error as SerdeDeError, Deserialize, Deserializer};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::result::Result as StdResult;
use std::str::FromStr;
use toml;

#[derive(Debug, Deserialize)]
pub struct SkyMetadata {
    #[serde(deserialize_with = "deserialize_name_from_str")]
    pub texture_name: WadName,
    #[serde(deserialize_with = "deserialize_regex_from_str")]
    pub level_pattern: Regex,
    pub tiled_band_size: f32,
}

#[derive(Debug, Deserialize)]
pub struct AnimationMetadata {
    #[serde(deserialize_with = "deserialize_name_from_vec_vec_str")]
    pub flats: Vec<Vec<WadName>>,
    #[serde(deserialize_with = "deserialize_name_from_vec_vec_str")]
    pub walls: Vec<Vec<WadName>>,
}

#[derive(Debug, Deserialize)]
pub struct ThingMetadata {
    pub thing_type: ThingType,
    #[serde(deserialize_with = "deserialize_name_from_str")]
    pub sprite: WadName,
    pub sequence: String,
    pub hanging: bool,
    pub radius: u32,
}

#[derive(Debug, Deserialize)]
pub struct ThingDirectoryMetadata {
    pub decorations: Vec<ThingMetadata>,
    pub weapons: Vec<ThingMetadata>,
    pub powerups: Vec<ThingMetadata>,
    pub artifacts: Vec<ThingMetadata>,
    pub ammo: Vec<ThingMetadata>,
    pub keys: Vec<ThingMetadata>,
    pub monsters: Vec<ThingMetadata>,
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub enum TriggerType {
    Any,
    Push,
    Switch,
    WalkOver,
    Gun,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum HeightRef {
    LowestFloor,
    NextFloor,
    HighestFloor,
    LowestCeiling,
    HighestCeiling,
    Floor,
    Ceiling,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightDef {
    pub to: HeightRef,

    #[serde(default = "Default::default", rename = "off")]
    pub offset: WadCoord,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightEffectDef {
    pub first: HeightDef,
    pub second: Option<HeightDef>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct MoveEffectDef {
    pub floor: Option<HeightEffectDef>,
    pub ceiling: Option<HeightEffectDef>,

    #[serde(default = "Default::default")]
    pub repeat: bool,

    #[serde(default = "Default::default")]
    pub wait: f32,

    #[serde(
        default = "Default::default",
        deserialize_with = "deserialize_move_speed"
    )]
    pub speed: f32,
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub enum ExitEffectDef {
    Normal,
    Secret,
}

#[derive(Debug, Deserialize)]
pub struct LinedefMetadata {
    pub special_type: SpecialType,
    pub trigger: TriggerType,

    #[serde(default = "Default::default")]
    pub monsters: bool,

    #[serde(default = "Default::default")]
    pub only_once: bool,

    #[serde(rename = "move")]
    pub move_effect: Option<MoveEffectDef>,

    #[serde(rename = "exit")]
    pub exit_effect: Option<ExitEffectDef>,
}

#[derive(Debug, Deserialize)]
pub struct WadMetadata {
    pub sky: Vec<SkyMetadata>,
    pub animations: AnimationMetadata,
    pub things: ThingDirectoryMetadata,

    #[serde(
        default = "Default::default",
        deserialize_with = "deserialize_linedefs"
    )]
    pub linedef: IndexMap<SpecialType, LinedefMetadata>,
}

impl WadMetadata {
    pub fn from_file<P: AsRef<Path>>(path: &P) -> Result<WadMetadata> {
        let mut contents = String::new();
        let path = path.as_ref();
        File::open(path)
            .and_then(|mut file| file.read_to_string(&mut contents))
            .chain_err(|| ErrorKind::on_metadata_read())?;
        WadMetadata::from_text(&contents)
    }

    pub fn from_text(text: &str) -> Result<WadMetadata> {
        toml::from_str(text).chain_err(ErrorKind::on_metadata_parse)
    }

    pub fn sky_for(&self, name: WadName) -> Option<&SkyMetadata> {
        self.sky
            .iter()
            .find(|sky| sky.level_pattern.is_match(name.as_ref()))
            .or_else(|| {
                if let Some(sky) = self.sky.get(0) {
                    warn!(
                        "No sky found for level {}, using {}.",
                        name, sky.texture_name
                    );
                    Some(sky)
                } else {
                    error!("No sky metadata provided.");
                    None
                }
            })
    }

    pub fn find_thing(&self, thing_type: ThingType) -> Option<&ThingMetadata> {
        self.things
            .decorations
            .iter()
            .find(|t| t.thing_type == thing_type)
            .or_else(|| {
                self.things
                    .weapons
                    .iter()
                    .find(|t| t.thing_type == thing_type)
            })
            .or_else(|| {
                self.things
                    .powerups
                    .iter()
                    .find(|t| t.thing_type == thing_type)
            })
            .or_else(|| {
                self.things
                    .artifacts
                    .iter()
                    .find(|t| t.thing_type == thing_type)
            })
            .or_else(|| self.things.ammo.iter().find(|t| t.thing_type == thing_type))
            .or_else(|| self.things.keys.iter().find(|t| t.thing_type == thing_type))
            .or_else(|| {
                self.things
                    .monsters
                    .iter()
                    .find(|t| t.thing_type == thing_type)
            })
    }
}

fn deserialize_regex_from_str<'de, D>(deserializer: D) -> StdResult<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    Regex::new(<&'de str>::deserialize(deserializer)?).map_err(D::Error::custom)
}

fn deserialize_name_from_str<'de, D>(deserializer: D) -> StdResult<WadName, D::Error>
where
    D: Deserializer<'de>,
{
    WadName::from_str(<&'de str>::deserialize(deserializer)?).map_err(D::Error::custom)
}

fn deserialize_move_speed<'de, D>(deserializer: D) -> StdResult<f32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(f32::deserialize(deserializer)? / 8.0 * 0.7)
}

fn deserialize_name_from_vec_vec_str<'de, D>(
    deserializer: D,
) -> StdResult<Vec<Vec<WadName>>, D::Error>
where
    D: Deserializer<'de>,
{
    let strings = <Vec<Vec<&'de str>>>::deserialize(deserializer)?;
    strings
        .iter()
        .map(|strings| {
            strings
                .iter()
                .map(|string| WadName::from_str(string))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<Vec<_>>>>()
        .map_err(D::Error::custom)
}

fn deserialize_linedefs<'de, D>(
    deserializer: D,
) -> StdResult<IndexMap<SpecialType, LinedefMetadata>, D::Error>
where
    D: Deserializer<'de>,
{
    let linedefs = <Vec<LinedefMetadata>>::deserialize(deserializer)?;
    Ok(linedefs
        .into_iter()
        .map(|linedef| (linedef.special_type, linedef))
        .collect::<IndexMap<_, _>>())
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
        )
        .expect("test: could not parse test metadata");
    }
}

use base;
use regex::Regex;
use rustc_serialize;
use super::name::WadName;
use super::types::ThingType;
use toml;
use toml::DecodeError;
use std::path::Path;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct SkyMetadata {
    pub texture_name: WadName,
    pub level_pattern: String,
    pub tiled_band_size: f32,
}


#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct AnimationMetadata {
    pub flats: Vec<Vec<WadName>>,
    pub walls: Vec<Vec<WadName>>,
}


#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct ThingMetadata {
    pub thing_type: ThingType,
    pub sprite: String,
    pub sequence: String,
    pub hanging: bool,
    pub radius: u32,
}


#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct ThingDirectoryMetadata {
    pub decorations: Vec<ThingMetadata>,
    pub weapons: Vec<ThingMetadata>,
    pub powerups: Vec<ThingMetadata>,
    pub artifacts: Vec<ThingMetadata>,
    pub ammo: Vec<ThingMetadata>,
    pub keys: Vec<ThingMetadata>,
    pub monsters: Vec<ThingMetadata>,
}


#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct WadMetadata {
    pub sky: Vec<SkyMetadata>,
    pub animations: AnimationMetadata,
    pub things: ThingDirectoryMetadata,
}
impl WadMetadata {
    pub fn from_file<P: AsRef<Path>>(path: &P) -> Result<WadMetadata, String> {
        base::read_utf8_file(path).and_then(
            |contents| WadMetadata::from_text(&contents))
    }

    pub fn from_text(text: &str) -> Result<WadMetadata, String> {
        let mut parser = toml::Parser::new(text);
        match parser.parse() {
            Some(value) => rustc_serialize::Decodable::decode(
                    &mut toml::Decoder::new(toml::Value::Table(value)))
                .map_err(|e| show_decode_err(e)),
            None => Err(format!("Error parsing WadMetadata from TOML: {:?}",
                                parser.errors))
        }
    }

    pub fn sky_for(&self, name: &WadName) -> &SkyMetadata {
        for sky in self.sky.iter() {
            let regex = Regex::new(&sky.level_pattern).unwrap();
            if regex.is_match(name.as_str()) {
                return sky;
            }
        }
        &self.sky[0]
    }
}

fn show_decode_err(err: DecodeError) -> String {
    use toml::DecodeErrorKind::*;

    format!("Error decoding WadMetadata: in field '{}': {}",
            err.field.unwrap_or("none".to_string()),
            match err.kind {
                ApplicationError(msg) => msg,
                ExpectedField(e) => format!("expected field '{}'", e.unwrap_or("none")),
                ExpectedType(e, f) => format!("expected type '{}', found '{}'", e, f),
                ExpectedMapKey(e) => format!("map key '{}' expected", e),
                ExpectedMapElement(e) => format!("map value '{}' expected", e),
                SyntaxError => format!("syntax error"),
                EndOfStream => format!("end of stream"),
                NoEnumVariants => format!("no enum variants"),
                NilTooLong => format!("non-empty string for nil type")
            })
}

#[cfg(test)]
mod test {
    use super::WadMetadata;

    #[test]
    fn test_wad_metadata() {
        WadMetadata::from_text(r#"
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
                [[things.decoration]]
                    thing_type = 10
                    sprite = "PLAY"
                    sequence = "W"
                    obstacle = false
                    hanging = false

                [[things.decoration]]
                    thing_type = 12
                    sprite = "PLAY"
                    sequence = "W"
                    obstacle = false
                    hanging = false
        "#).unwrap();
    }
}


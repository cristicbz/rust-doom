use base;
use regex::Regex;
use serialize;
use super::name::WadName;
use super::types::ThingType;
use toml;
use toml::{DecodeError, ApplicationError, ExpectedField, ExpectedType,
           ExpectedMapElement, ExpectedMapKey, NoEnumVariants, NilTooLong};


#[deriving(Decodable, Encodable)]
pub struct SkyMetadata {
    pub texture_name: WadName,
    pub level_pattern: String,
    pub tiled_band_size: f32,
}


#[deriving(Decodable, Encodable)]
pub struct AnimationMetadata {
    pub flats: Vec<Vec<WadName>>,
    pub walls: Vec<Vec<WadName>>,
}


#[deriving(Decodable, Encodable)]
pub struct ThingMetadata {
    pub thing_type: ThingType,
    pub sprite: WadName,
    pub sequence: String,
}


#[deriving(Decodable, Encodable)]
pub struct ThingDirectoryMetadata {
    pub decoration: Vec<ThingMetadata>
}


#[deriving(Decodable, Encodable)]
pub struct WadMetadata {
    pub sky: Vec<SkyMetadata>,
    pub animations: AnimationMetadata,
    pub things: ThingDirectoryMetadata,
}
impl WadMetadata {
    pub fn from_file(path: &Path) -> Result<WadMetadata, String> {
        base::read_utf8_file(path).and_then(
            |contents| WadMetadata::from_text(contents[]))
    }

    pub fn from_text(text: &str) -> Result<WadMetadata, String> {
        let mut parser = toml::Parser::new(text);
        match parser.parse() {
            Some(value) => serialize::Decodable::decode(
                               &mut toml::Decoder::new(toml::Table(value))
                           ).map_err(|e| show_decode_err(e)),
            None => Err(format!("Error parsing WadMetadata from TOML: {}",
                                parser.errors))
        }
    }

    pub fn sky_for(&self, name: &WadName) -> &SkyMetadata {
        for sky in self.sky.iter() {
            let regex = Regex::new(sky.level_pattern[]).unwrap();
            if regex.is_match(name.as_str()) {
                return sky;
            }
        }
        &self.sky[0]
    }
}

fn show_decode_err(err: DecodeError) -> String {
    format!("Error decoding WadMetadata: in field '{}': {}",
            err.field.unwrap_or("none".to_string()),
            match err.kind {
                ApplicationError(msg) => msg,
                ExpectedField(e) => format!("expected field '{}'", e),
                ExpectedType(e, f) => format!("expected type '{}', found '{}'",
                                              e, f),
                ExpectedMapKey(e) => format!("map key '{}' expected", e),
                ExpectedMapElement(e) => format!("map value '{}' expected", e),
                NoEnumVariants => format!("no enum variants"),
                NilTooLong => format!("non-empty string for nil type")
            })
}

#[cfg(test)]
mod test {
    use super::WadMetadata;

    #[test]
    fn test_wad_metadata() {
        assert!("{}", WadMetadata::from_text(r#"
            [[sky]]
                level_range = [0, 10]
                texture_name = "SKY1"
                tiled_band_size = 0.15
            [[sky]]
                level_range = [11, 20]
                texture_name = "SKY2"
                tiled_band_size = 0.15
            [[sky]]
                level_range = [21, 35]
                texture_name = "SKY3"
                tiled_band_size = 0.15
        "#).is_ok());
    }
}


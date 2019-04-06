use failchain::{BoxedError, ChainErrorKind};
use failure::Fail;
use std::fmt;
use std::result::Result as StdResult;

pub type Error = BoxedError<ErrorKind>;
pub type Result<T> = StdResult<T, Error>;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Corrupt metadata file: {}", 0)]
    CorruptMetadata(String),

    #[fail(display = "Corrupt WAD file: {}", 0)]
    CorruptWad(String),

    #[fail(display = "I/O WAD error: {}", 0)]
    Io(String),
}

impl ChainErrorKind for ErrorKind {
    type Error = Error;
}

impl ErrorKind {
    pub(crate) fn invalid_byte_in_wad_name(byte: u8, bytes: &[u8]) -> Self {
        ErrorKind::CorruptWad(format!(
            "Invalid character `{}` in wad name `{}`.",
            char::from(byte),
            String::from_utf8_lossy(bytes),
        ))
    }

    pub(crate) fn unfinished_image_column(
        i_column: usize,
        i_run: Option<usize>,
        width: usize,
        height: usize,
    ) -> Self {
        ErrorKind::CorruptWad(format!(
            "Unfinished column {} in run {:?}, in image of size {}x{}",
            i_column, i_run, width, height
        ))
    }

    pub(crate) fn image_too_large(width: usize, height: usize) -> Self {
        ErrorKind::CorruptWad(format!("Image too large {}x{}.", width, height))
    }

    pub(crate) fn wad_name_too_long(bytes: &[u8]) -> Self {
        ErrorKind::CorruptWad(format!(
            "Wad name too long `{}`.",
            String::from_utf8_lossy(bytes)
        ))
    }

    pub(crate) fn bad_wad_header() -> Self {
        ErrorKind::CorruptWad("Could not read WAD header.".to_owned())
    }

    pub(crate) fn bad_wad_header_identifier(identifier: &[u8]) -> Self {
        ErrorKind::CorruptWad(format!(
            "Invalid header identifier: {}",
            String::from_utf8_lossy(identifier)
        ))
    }

    pub(crate) fn on_metadata_read() -> Self {
        ErrorKind::Io("Failed to load metadata to memory.".to_owned())
    }

    pub(crate) fn on_metadata_parse() -> Self {
        ErrorKind::CorruptMetadata("Failed to parse metadata file.".to_owned())
    }

    pub(crate) fn on_file_open() -> Self {
        ErrorKind::Io("Failed to open file.".to_owned())
    }

    pub(crate) fn seeking_to_info_table_offset(offset: i32) -> Self {
        ErrorKind::Io(format!(
            "Seeking to `info_table_offset` at {} failed",
            offset
        ))
    }

    pub(crate) fn seeking_to_lump(index: usize, name: &str) -> Self {
        ErrorKind::Io(format!("Seeking to lump {}, `{}` failed", index, name))
    }

    pub(crate) fn reading_lump(index: usize, name: &str) -> Self {
        ErrorKind::Io(format!("Reading lump {}, `{}` failed", index, name))
    }

    pub(crate) fn bad_lump_info(lump_index: i32) -> Self {
        ErrorKind::CorruptWad(format!("Invalid lump info for lump {}", lump_index))
    }

    pub(crate) fn bad_lump_element(
        lump_index: usize,
        lump_name: &str,
        element_index: usize,
    ) -> Self {
        ErrorKind::CorruptWad(format!(
            "Invalid element {} in lump `{}` (index={})",
            element_index, lump_name, lump_index
        ))
    }

    pub(crate) fn bad_lump_size(
        index: usize,
        name: &str,
        total_size: usize,
        element_size: usize,
    ) -> Self {
        ErrorKind::CorruptWad(format!(
            "Invalid lump size in `{}` (index={}): total={}, element={}, div={}, mod={}",
            name,
            index,
            total_size,
            element_size,
            total_size / element_size,
            total_size % element_size
        ))
    }

    pub(crate) fn missing_required_lump<NameT: fmt::Debug>(name: &NameT) -> Self {
        ErrorKind::CorruptWad(format!("Missing required lump {:?}", name))
    }
}

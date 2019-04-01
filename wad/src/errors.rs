use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Corrupt metadata file: {}", 0)]
    CorruptMetadata(String),

    #[fail(display = "Corrupt WAD file: {}", 0)]
    CorruptWad(String),

    #[fail(display = "WAD image error: {}", 0)]
    ImageError(String),

    #[fail(display = "I/O WAD error: {}", 0)]
    Io(String),
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }

    pub(crate) fn invalid_byte_in_wad_name(byte: u8, bytes: &[u8]) -> Self {
        Self::from(ErrorKind::CorruptWad(format!(
            "Invalid character `{}` in wad name `{}`.",
            char::from(byte),
            String::from_utf8_lossy(bytes),
        )))
    }

    pub(crate) fn image_context<StringT: Into<String>, ErrorT: Fail>(
        message: StringT,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error: ErrorT| Self::from(error.context(ErrorKind::ImageError(message.into())))
    }

    pub(crate) fn unfinished_image_column<ErrorT: Fail>(
        i_column: usize,
        width: usize,
        height: usize,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error: ErrorT| {
            Self::from(error.context(ErrorKind::ImageError(format!(
                "unfinished column {}, {}x{}",
                i_column, width, height
            ))))
        }
    }

    pub(crate) fn image_too_large(width: usize, height: usize) -> Self {
        Self::image(format!("Image too large {}x{}.", width, height))
    }

    pub(crate) fn image<StringT: Into<String>>(message: StringT) -> Self {
        Self::from(ErrorKind::ImageError(message.into()))
    }

    pub(crate) fn wad_name_too_long(bytes: &[u8]) -> Self {
        Self::from(ErrorKind::CorruptWad(format!(
            "Wad name too long `{}`.",
            String::from_utf8_lossy(bytes)
        )))
    }

    pub(crate) fn bad_wad_header<ErrorT: Fail>(error: ErrorT) -> Self {
        Self::from(error.context(ErrorKind::CorruptWad(
            "Could not read WAD header.".to_owned(),
        )))
    }

    pub(crate) fn bad_wad_header_identifier(identifier: &[u8]) -> Self {
        Self::from(ErrorKind::CorruptWad(format!(
            "Invalid header identifier: {}",
            String::from_utf8_lossy(identifier)
        )))
    }

    pub(crate) fn on_metadata_read<ErrorT: Fail>(error: ErrorT) -> Self {
        Self::from(error.context(ErrorKind::Io(
            "Failed to load metadata to memory.".to_owned(),
        )))
    }

    pub(crate) fn on_metadata_parse<ErrorT: Fail>(error: ErrorT) -> Self {
        Self::from(error.context(ErrorKind::CorruptMetadata(
            "Failed to parse metadata file.".to_owned(),
        )))
    }

    pub(crate) fn on_file_open<ErrorT: Fail>(error: ErrorT) -> Self {
        Self::from(error.context(ErrorKind::Io("Failed to open file.".to_owned())))
    }

    pub(crate) fn seeking_to_info_table_offset<ErrorT: Fail>(
        offset: i32,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error: ErrorT| {
            Self::from(error.context(ErrorKind::Io(format!(
                "Seeking to `info_table_offset` at {} failed",
                offset
            ))))
        }
    }

    pub(crate) fn seeking_to_lump<'a, ErrorT: Fail>(
        index: usize,
        name: &'a str,
    ) -> (impl FnOnce(ErrorT) -> Self + 'a) {
        move |error: ErrorT| {
            Self::from(error.context(ErrorKind::Io(format!(
                "Seeking to lump {}, `{}` failed",
                index, name
            ))))
        }
    }

    pub(crate) fn reading_lump<'a, ErrorT: Fail>(
        index: usize,
        name: &'a str,
    ) -> (impl FnOnce(ErrorT) -> Self + 'a) {
        move |error: ErrorT| {
            Self::from(error.context(ErrorKind::Io(format!(
                "Reading lump {}, `{}` failed",
                index, name
            ))))
        }
    }

    pub(crate) fn bad_lump_info<ErrorT: Fail>(lump_index: i32) -> (impl FnOnce(ErrorT) -> Self) {
        move |error: ErrorT| {
            Self::from(error.context(ErrorKind::CorruptWad(format!(
                "Invalid lump info for lump {}",
                lump_index
            ))))
        }
    }

    pub(crate) fn missing_number_of_patches<ErrorT: Fail>(error: ErrorT) -> Self {
        Self::from(error.context(ErrorKind::CorruptWad(
            "Missing number of patches in PNAMES".to_owned(),
        )))
    }

    pub(crate) fn missing_number_of_textures<ErrorT: Fail>(error: ErrorT) -> Self {
        Self::from(error.context(ErrorKind::CorruptWad(
            "Missing number of textures".to_owned(),
        )))
    }

    pub(crate) fn textures_lump_too_small_for_offsets(lump_len: usize, offsets_end: usize) -> Self {
        Self::from(ErrorKind::CorruptWad(format!(
            "Textures lump too small for offsets: {} < {}",
            lump_len, offsets_end
        )))
    }

    pub(crate) fn bad_lump_element<'a, ErrorT: Fail>(
        lump_index: usize,
        lump_name: &'a str,
        element_index: usize,
    ) -> (impl FnOnce(ErrorT) -> Self + 'a) {
        move |error: ErrorT| {
            Self::from(error.context(ErrorKind::CorruptWad(format!(
                "Invalid element {} in lump `{}` (index={})",
                element_index, lump_name, lump_index
            ))))
        }
    }

    pub(crate) fn bad_lump_size(
        index: usize,
        name: &str,
        total_size: usize,
        element_size: usize,
    ) -> Self {
        Self::from(ErrorKind::CorruptWad(format!(
            "Invalid lump size in `{}` (index={}): total={}, element={}, div={}, mod={}",
            name,
            index,
            total_size,
            element_size,
            total_size / element_size,
            total_size % element_size
        )))
    }

    pub(crate) fn missing_required_lump<NameT: fmt::Debug>(name: &NameT) -> Self {
        Self::from(ErrorKind::CorruptWad(format!(
            "Missing required lump {:?}",
            name
        )))
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self::from(Context::new(kind))
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self {
        Error { inner }
    }
}

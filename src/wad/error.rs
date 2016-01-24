use byteorder::Error as ByteOrderError;
use image::ImageError;
use name::WadName;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use toml::DecodeError as TomlDecodeError;
use toml::ParserError as TomlParserError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub struct Error {
    file: Option<PathBuf>,
    kind: ErrorKind,
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.kind.description()
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match self.file {
            Some(ref path) => write!(fmt, "in '{}': {}", path.to_string_lossy(), self.kind),
            None => write!(fmt, "{}", self.kind),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Io(IoError),
    ByteOrder(ByteOrderError),
    BadWadHeader,
    BadWadName(Vec<u8>),
    MissingRequiredLump(String),
    // MissingRequiredPatch(WadName, WadName),
    BadMetadataSchema(TomlDecodeError),
    BadMetadataSyntax(Vec<TomlParserError>),
    BadImage(WadName, ImageError),
}

impl ErrorKind {
    fn description(&self) -> &str {
        match *self {
            ErrorKind::Io(ref inner) => inner.description(),
            ErrorKind::ByteOrder(ref inner) => inner.description(),
            ErrorKind::BadWadHeader => "invalid header",
            ErrorKind::BadWadName(..) => "invalid wad name",
            ErrorKind::MissingRequiredLump(..) => "missing required lump",
            // ErrorKind::MissingRequiredPatch(..) => "missing required patch",
            ErrorKind::BadMetadataSchema(..) => "invalid data in metadata",
            ErrorKind::BadMetadataSyntax(..) => "TOML syntax error in metadata",
            ErrorKind::BadImage(..) => "Bad image",
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let desc = self.description();
        match *self {
            ErrorKind::Io(ref inner) => write!(fmt, "{}", inner),
            ErrorKind::ByteOrder(ref inner) => write!(fmt, "{}", inner),
            ErrorKind::BadWadHeader => write!(fmt, "{}", desc),
            ErrorKind::BadWadName(ref name) => write!(fmt, "{} ({:?})", desc, name),
            ErrorKind::MissingRequiredLump(ref name) => write!(fmt, "{} ({})", desc, name),
            // ErrorKind::MissingRequiredPatch(ref patch, ref texture) => {
            //    write!(fmt, "{} ({}, required by {})", desc, patch, texture)
            // },
            ErrorKind::BadMetadataSchema(ref err) => write!(fmt, "{}: {}", desc, err),
            ErrorKind::BadMetadataSyntax(ref errors) => write!(fmt, "{}: {:?}", desc, errors),
            ErrorKind::BadImage(ref name, ref inner) => {
                write!(fmt, "{}: in {}: {}", desc, name, inner)
            }
        }
    }
}

impl From<IoError> for Error {
    fn from(cause: IoError) -> Error {
        ErrorKind::Io(cause).into()
    }
}

impl From<ByteOrderError> for Error {
    fn from(cause: ByteOrderError) -> Error {
        ErrorKind::ByteOrder(cause).into()
    }
}

impl From<TomlDecodeError> for Error {
    fn from(cause: TomlDecodeError) -> Error {
        ErrorKind::BadMetadataSchema(cause).into()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind: kind,
            file: None,
        }
    }
}

pub trait InFile {
    type Output;
    fn in_file(self, file: &Path) -> Self::Output;
}

impl InFile for Error {
    type Output = Error;
    fn in_file(self, file: &Path) -> Error {
        Error {
            file: Some(file.to_owned()),
            kind: self.kind,
        }
    }
}

impl InFile for ErrorKind {
    type Output = Error;
    fn in_file(self, file: &Path) -> Error {
        Error {
            file: Some(file.to_owned()),
            kind: self,
        }
    }
}

impl<S, E: Into<Error>> InFile for StdResult<S, E> {
    type Output = Result<S>;
    fn in_file(self, file: &Path) -> Result<S> {
        self.map_err(|e| e.into().in_file(file))
    }
}

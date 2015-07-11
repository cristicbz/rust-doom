use std::error::Error as StdError;
use std::fmt::{Formatter, Display};
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Sdl(String),
    VertexCompile {
        shader: String,
        log: String,
    },
    FragmentCompile {
        shader: String,
        log: String,
    },
    Link {
        shader: String,
        log: String,
    },
    NoSuchUniform {
        shader: String,
        uniform: String,
    },
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref inner) => inner.description(),
            Error::Sdl(ref inner) => &inner[..],
            Error::VertexCompile { .. } => "vertex shader compilation error",
            Error::FragmentCompile { .. } => "fragment shader compilation error",
            Error::Link { .. } => "shader linking error",
            Error::NoSuchUniform { .. } => "no such uniform",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error { Error::Io(err) }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        try!(write!(fmt, "graphics error: "));
        match *self {
            Error::Io(ref e) => write!(fmt, "I/O: {}", e),
            Error::Sdl(ref e) => write!(fmt, "SDL: {}", e),
            Error::VertexCompile { ref shader, ref log } => {
                write!(fmt, "in vertex shader of '{}': {}", shader, log)
            },
            Error::FragmentCompile { ref shader, ref log } => {
                write!(fmt, "in fragment shader of '{}': {}", shader, log)
            },
            Error::Link { ref shader, ref log } => {
                write!(fmt, "in shader link phase for '{}': {}", shader, log)
            },
            Error::NoSuchUniform { ref shader, ref uniform } => {
                write!(fmt, "in shader '{}': no such uniform '{}'", shader, uniform)
            },
        }
    }
}

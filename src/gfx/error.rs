use std::error::Error as StdError;
use std::fmt::{Formatter, Display};
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::result::Result as StdResult;
use sdl2::ErrorMessage as SdlError;
use glium::GliumCreationError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Sdl(SdlError),
    IncompatibleOpenGl(String),
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
            Error::Sdl(ref inner) => &inner.0[..],
            Error::IncompatibleOpenGl(ref inner) => &inner[..],
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

impl From<SdlError> for Error {
    fn from(err: SdlError) -> Error { Error::Sdl(err) }
}

impl From<GliumCreationError<SdlError>> for Error {
    fn from(err: GliumCreationError<SdlError>) -> Error {
        match err {
           GliumCreationError::BackendCreationError(err) => Error::Sdl(err),
           GliumCreationError::IncompatibleOpenGl(msg) => Error::IncompatibleOpenGl(msg),
        }
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        try!(write!(fmt, "graphics error: "));
        match *self {
            Error::Io(ref e) => write!(fmt, "I/O: {}", e),
            Error::Sdl(ref e) => write!(fmt, "SDL: {}", e),
            Error::IncompatibleOpenGl(ref e) => write!(fmt, "Incompatible OpenGL: {}", e),
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

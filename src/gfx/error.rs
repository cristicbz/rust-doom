use std::error::Error as StdError;
use std::fmt::{Formatter, Display};
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    VertexCompile(String),
    FragmentCompile(String),
    Link(String),
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref inner) => inner.description(),
            Error::VertexCompile(_) => "vertex shader compilation error",
            Error::FragmentCompile(_) => "fragment shader compilation error",
            Error::Link(_) => "shader linking error",
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
            Error::Io(ref e) => try!(write!(fmt, "I/O: {}", e)),
            Error::VertexCompile(ref log) => try!(write!(fmt, "in vertex shader: {}", log)),
            Error::FragmentCompile(ref log) => try!(write!(fmt, "in fragment shader: {}", log)),
            Error::Link(ref log) => try!(write!(fmt, "in shader link phase: {}", log)),
        }
        Ok(())
    }
}

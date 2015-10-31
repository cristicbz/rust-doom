use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::result::Result as StdResult;
use sdl2::ErrorMessage as SdlError;
use glium::{self, GliumCreationError};

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Sdl(SdlError),
    Shader {
        log: String,
        needed_by: String,
    },
    IncompatibleOpenGl(String),
    UnsupportedFeature {
        feature: String,
        needed_by: String,
    },
    OutOfVideoMemory {
        needed_by: String,
    },
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref inner) => inner.description(),
            Error::Sdl(ref inner) => &inner.0[..],
            Error::IncompatibleOpenGl(ref inner) => &inner[..],
            Error::Shader { .. } => "shader compilation/linking error",
            Error::UnsupportedFeature { .. } => "unsupported required feature",
            Error::OutOfVideoMemory { .. } => "out of video memory",
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
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl From<SdlError> for Error {
    fn from(err: SdlError) -> Error {
        Error::Sdl(err)
    }
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
            Error::Shader { ref log, ref needed_by } =>
                write!(fmt, "Error building shader '{}': {}", needed_by, log),
            Error::UnsupportedFeature { ref feature, ref needed_by } =>
                write!(fmt,
                       "Unsupported OpenGL feature '{}', required by '{}'",
                       feature,
                       needed_by),
            Error::OutOfVideoMemory { ref needed_by } =>
                write!(fmt,
                       "Out of video memory when try to allocate '{}'",
                       needed_by),
        }
    }
}

pub trait NeededBy: Sized {
    type Success;

    fn needed_by(self, by: &str) -> Result<Self::Success>;
}

impl<S> NeededBy for StdResult<S, glium::vertex::BufferCreationError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        self.map_err(|e| {
            Error::UnsupportedFeature {
                feature: e.to_string(),
                needed_by: by.to_owned(),
            }
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::texture::TextureCreationError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        self.map_err(|e| {
            Error::UnsupportedFeature {
                needed_by: by.to_owned(),
                feature: format!("{:?}", e),
            }
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::texture::buffer_texture::CreationError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        use glium::texture::buffer_texture::CreationError::*;
        use glium::texture::buffer_texture::TextureCreationError::*;
        use glium::buffer::BufferCreationError::*;
        self.map_err(|e| {
            match e {
                BufferCreationError(OutOfMemory) =>
                    Error::OutOfVideoMemory { needed_by: by.to_owned() },
                e@TextureCreationError(FormatNotSupported) |
                e@TextureCreationError(NotSupported) |
                e@TextureCreationError(TooLarge) |
                e@BufferCreationError(BufferTypeNotSupported) => Error::UnsupportedFeature {
                    feature: format!("{:?}", e),
                    needed_by: by.to_owned(),
                },
            }
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::ProgramCreationError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        use glium::ProgramCreationError::*;
        self.map_err(|e| {
            match e {
                CompilationError(log) |
                LinkingError(log) => Error::Shader {
                    needed_by: by.to_owned(),
                    log: log,
                },

                e@ShaderTypeNotSupported |
                e@CompilationNotSupported |
                e@TransformFeedbackNotSupported |
                e@PointSizeNotSupported => Error::UnsupportedFeature {
                    feature: e.to_string(),
                    needed_by: by.to_owned(),
                },
            }
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::DrawError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        use glium::DrawError::*;
        self.map_err(|e| {
            match e {
                e@ViewportTooLarge |
                e@UnsupportedVerticesPerPatch |
                e@TessellationNotSupported |
                e@SamplersNotSupported |
                e@TransformFeedbackNotSupported |
                e@SmoothingNotSupported |
                e@ProvokingVertexNotSupported |
                e@RasterizerDiscardNotSupported |
                e@DepthClampNotSupported |
                e@BlendingParameterNotSupported => Error::UnsupportedFeature {
                    feature: format!("{:?}", e),
                    needed_by: by.to_owned(),
                },

                e@NoDepthBuffer |
                e@AttributeTypeMismatch |
                e@AttributeMissing |
                e@InvalidDepthRange |
                e@UniformTypeMismatch { .. } |
                e@UniformBufferToValue { .. } |
                e@UniformValueToBlock { .. } |
                e@UniformBlockLayoutMismatch { .. } |
                e@TessellationWithoutPatches |
                e@InstancesCountMismatch |
                e@VerticesSourcesLengthMismatch |
                e@WrongQueryOperation => panic!("Invalid draw call: {:?}", e),
            }
        })
    }
}

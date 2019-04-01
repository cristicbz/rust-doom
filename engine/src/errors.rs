use failure::{Backtrace, Context, Fail};
use glium;
use idcontain::Id;
use std::fmt;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "{}", 0)]
    CreateWindow(String),

    #[fail(
        display = "I/O error when accessing `{}` resource `{}`.",
        kind, needed_by
    )]
    ResourceIo {
        kind: &'static str,
        needed_by: String,
    },

    #[fail(
        display = "Linking/compiling shader for `{}` failed with:\n{}",
        needed_by, log
    )]
    Shader { log: String, needed_by: String },

    #[fail(
        display = "Feature needed by `{}` is not supported on this platform.",
        needed_by
    )]
    UnsupportedFeature { needed_by: String },

    #[fail(
        display = "Out of video memory when trying to allocate `{}`.",
        needed_by
    )]
    OutOfVideoMemory { needed_by: String },

    #[fail(
        display = "No entity with id `{:?}`, needed by `{:?}` when `{}`",
        id, needed_by, context
    )]
    NoSuchEntity {
        context: &'static str,
        needed_by: Option<&'static str>,
        id: Id<()>,
    },

    #[fail(
        display = "No component with id `{:?}`, needed by `{:?}` when `{}`",
        id, needed_by, context
    )]
    NoSuchComponent {
        context: &'static str,
        needed_by: Option<&'static str>,
        id: Id<()>,
    },

    #[fail(display = "Context setup failed")]
    ContextSetup,

    #[fail(display = "Context update failed")]
    ContextUpdate,

    #[fail(display = "Context teardown failed")]
    ContextTeardown,

    #[fail(display = "Context destruction failed")]
    ContextDestruction,

    #[fail(display = "System `{}` creation failed", 0)]
    SystemCreation(&'static str),

    #[fail(display = "System `{}` setup failed", 0)]
    SystemSetup(&'static str),

    #[fail(display = "System `{}` update failed", 0)]
    SystemUpdate(&'static str),

    #[fail(display = "System `{}` teardown failed", 0)]
    SystemTeardown(&'static str),

    #[fail(display = "System `{}` destruction failed", 0)]
    SystemDestruction(&'static str),

    #[fail(display = "{}", 0)]
    Font(String),
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }

    pub(crate) fn context_setup<ErrorT: Fail>(error: ErrorT) -> Self {
        Error::from(error.context(ErrorKind::ContextSetup))
    }

    pub(crate) fn context_update<ErrorT: Fail>(error: ErrorT) -> Self {
        Error::from(error.context(ErrorKind::ContextUpdate))
    }

    pub(crate) fn context_teardown<ErrorT: Fail>(error: ErrorT) -> Self {
        Error::from(error.context(ErrorKind::ContextTeardown))
    }

    pub(crate) fn context_destruction<ErrorT: Fail>(error: ErrorT) -> Self {
        Error::from(error.context(ErrorKind::ContextDestruction))
    }

    pub(crate) fn system_creation<ErrorT: Fail>(
        system: &'static str,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| Error::from(error.context(ErrorKind::SystemCreation(system)))
    }

    pub(crate) fn system_setup<ErrorT: Fail>(
        system: &'static str,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| Error::from(error.context(ErrorKind::SystemSetup(system)))
    }

    pub(crate) fn system_update<ErrorT: Fail>(
        system: &'static str,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| Error::from(error.context(ErrorKind::SystemSetup(system)))
    }

    pub(crate) fn system_teardown<ErrorT: Fail>(
        system: &'static str,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| Error::from(error.context(ErrorKind::SystemSetup(system)))
    }

    pub(crate) fn system_destruction<ErrorT: Fail>(
        system: &'static str,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| Error::from(error.context(ErrorKind::SystemSetup(system)))
    }

    pub(crate) fn create_window(
        width: u32,
        height: u32,
    ) -> (impl FnOnce(glium::backend::glutin::DisplayCreationError) -> Self) {
        move |error| {
            Self::from(ErrorKind::CreateWindow(format!(
                "Window creation failed with {}x{}: {}",
                width, height, error
            )))
        }
    }

    pub(crate) fn resource_io<NeededByT: Into<String>, ErrorT: Fail>(
        kind: &'static str,
        needed_by: NeededByT,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| {
            Self::from(error.context(ErrorKind::ResourceIo {
                kind,
                needed_by: needed_by.into(),
            }))
        }
    }

    pub(crate) fn no_such_entity(
        context: &'static str,
        needed_by: Option<&'static str>,
        id: Id<()>,
    ) -> Self {
        Self::from(ErrorKind::NoSuchEntity {
            context,
            needed_by,
            id,
        })
    }

    pub(crate) fn no_such_component(
        context: &'static str,
        needed_by: Option<&'static str>,
        id: Id<()>,
    ) -> Self {
        Self::from(ErrorKind::NoSuchComponent {
            context,
            needed_by,
            id,
        })
    }

    pub(crate) fn glium<NeededByT: Into<String>, ErrorT: ConvertGlium>(
        needed_by: NeededByT,
    ) -> (impl FnOnce(ErrorT) -> Self) {
        move |error| error.convert_glium(needed_by.into())
    }

    pub(crate) fn font(error: impl Fail, message: impl Into<String>) -> Self {
        Error::from(error.context(ErrorKind::Font(message.into())))
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

pub(crate) trait ConvertGlium: Fail + Sized {
    fn convert_glium(self, needed_by: String) -> Error {
        Error::from(self.context(ErrorKind::UnsupportedFeature { needed_by }))
    }
}

impl ConvertGlium for glium::vertex::BufferCreationError {}
impl ConvertGlium for glium::index::BufferCreationError {}
impl ConvertGlium for glium::texture::TextureCreationError {}

impl ConvertGlium for glium::texture::buffer_texture::CreationError {
    fn convert_glium(self, needed_by: String) -> Error {
        use glium::buffer::BufferCreationError::*;
        use glium::texture::buffer_texture::CreationError::*;
        use glium::texture::buffer_texture::TextureCreationError::*;

        Error::from(self.context(match self {
            BufferCreationError(OutOfMemory) => ErrorKind::OutOfVideoMemory { needed_by },
            TextureCreationError(FormatNotSupported)
            | TextureCreationError(NotSupported)
            | TextureCreationError(TooLarge)
            | BufferCreationError(BufferTypeNotSupported) => {
                ErrorKind::UnsupportedFeature { needed_by }
            }
        }))
    }
}

impl ConvertGlium for glium::ProgramCreationError {
    fn convert_glium(self, needed_by: String) -> Error {
        use glium::ProgramCreationError::*;
        let kind = match &self {
            CompilationError(log) | LinkingError(log) => ErrorKind::Shader {
                log: log.clone(),
                needed_by,
            },

            BinaryHeaderError => ErrorKind::Shader {
                log: "Binary header error.".to_owned(),
                needed_by,
            },

            ShaderTypeNotSupported
            | CompilationNotSupported
            | TransformFeedbackNotSupported
            | PointSizeNotSupported => ErrorKind::UnsupportedFeature { needed_by },
        };
        Error::from(self.context(kind))
    }
}

impl ConvertGlium for glium::DrawError {
    fn convert_glium(self, needed_by: String) -> Error {
        use glium::DrawError::*;
        let kind = match self {
            FixedIndexRestartingNotSupported
            | ViewportTooLarge
            | UnsupportedVerticesPerPatch
            | TessellationNotSupported
            | SamplersNotSupported
            | TransformFeedbackNotSupported
            | SmoothingNotSupported
            | ProvokingVertexNotSupported
            | RasterizerDiscardNotSupported
            | DepthClampNotSupported
            | BlendingParameterNotSupported => ErrorKind::UnsupportedFeature { needed_by },

            NoDepthBuffer
            | AttributeTypeMismatch
            | AttributeMissing
            | InvalidDepthRange
            | UniformTypeMismatch { .. }
            | UniformBufferToValue { .. }
            | UniformValueToBlock { .. }
            | UniformBlockLayoutMismatch { .. }
            | TessellationWithoutPatches
            | InstancesCountMismatch
            | VerticesSourcesLengthMismatch
            | SubroutineNotFound { .. }
            | SubroutineUniformMissing { .. }
            | SubroutineUniformToValue { .. }
            | ClipPlaneIndexOutOfBounds { .. }
            | WrongQueryOperation => panic!("Invalid draw call: {:?}", self),
        };
        Error::from(self.context(kind))
    }
}

use failchain::{BoxedError, ChainErrorKind};
use failure::Fail;
use glium;
use idcontain::Id;
use std::result::Result as StdResult;

pub type Error = BoxedError<ErrorKind>;
pub type Result<T> = StdResult<T, Error>;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "{}", 0)]
    CreateWindow(String),

    #[fail(display = "I/O error when accessing `{}` for resource `{}`.", 0, 1)]
    ResourceIo(&'static str, &'static str),

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

    #[fail(display = "Context {} error", 0)]
    Context(&'static str),

    #[fail(display = "System {} failed for `{}`.", 0, 1)]
    System(&'static str, &'static str),
}

impl ChainErrorKind for ErrorKind {
    type Error = Error;
}

impl ErrorKind {
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

    pub(crate) fn glium<NeededByT: Into<String>, ErrorT: ConvertGlium>(
        needed_by: NeededByT,
    ) -> (impl FnOnce(ErrorT) -> Error) {
        move |error| error.convert_glium(needed_by.into()).into()
    }
}

pub(crate) trait ConvertGlium: Fail + Sized {
    fn convert_glium(self, needed_by: String) -> ErrorKind;
}

pub(crate) trait UnsupportedFeature: Fail + Sized {}

impl<T: UnsupportedFeature> ConvertGlium for T {
    fn convert_glium(self, needed_by: String) -> ErrorKind {
        ErrorKind::UnsupportedFeature { needed_by }
    }
}

impl UnsupportedFeature for glium::vertex::BufferCreationError {}
impl UnsupportedFeature for glium::index::BufferCreationError {}
impl UnsupportedFeature for glium::texture::TextureCreationError {}

impl ConvertGlium for glium::texture::buffer_texture::CreationError {
    fn convert_glium(self, needed_by: String) -> ErrorKind {
        use glium::buffer::BufferCreationError::*;
        use glium::texture::buffer_texture::CreationError::*;
        use glium::texture::buffer_texture::TextureCreationError::*;

        match self {
            BufferCreationError(OutOfMemory) => ErrorKind::OutOfVideoMemory { needed_by },
            TextureCreationError(FormatNotSupported)
            | TextureCreationError(NotSupported)
            | TextureCreationError(TooLarge)
            | BufferCreationError(BufferTypeNotSupported) => {
                ErrorKind::UnsupportedFeature { needed_by }
            }
        }
    }
}

impl ConvertGlium for glium::ProgramCreationError {
    fn convert_glium(self, needed_by: String) -> ErrorKind {
        use glium::ProgramCreationError::*;
        match &self {
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
        }
    }
}

impl ConvertGlium for glium::DrawError {
    fn convert_glium(self, needed_by: String) -> ErrorKind {
        use glium::DrawError::*;
        match self {
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
        }
    }
}

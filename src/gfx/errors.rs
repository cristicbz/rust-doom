use glium;
use std::result::Result as StdResult;

error_chain! {
    errors {
        CreateWindow(width: u32, height: u32) {
            description("Window creation failed.")
            display("Window creation failed with {}x{}", width, height)
        }
        Sdl(message: String) {
            description("SDL Error.")
            display("SDL Error: {}", message)
        }
        ResourceIo(kind: &'static str, needed_by: String) {
            description("I/O error when accessing resource.")
            display("I/O error when accessing `{}` resource `{}`.", kind, needed_by)
        }
        Shader(log: String, needed_by: String) {
            description("Shader compilation error.")
            display("Linking/compiling shader for `{}` failed with:\n{}", needed_by, log)
        }
        UnsupportedFeature(feature: String, needed_by: String) {
            description("Unsupported graphics feature.")
            display("Feature `{}` needed by `{}` is not supported on this platform.",
                    feature, needed_by)
        }
        OutOfVideoMemory(needed_by: String) {
            description("Out of video memory.")
            display("Out of video memory when trying to allocate `{}`.", needed_by)
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
            ErrorKind::UnsupportedFeature(e.to_string(), by.to_owned()).into()
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::texture::TextureCreationError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        self.map_err(|e| {
            ErrorKind::UnsupportedFeature(e.to_string(), by.to_owned()).into()
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
            (match e {
                 BufferCreationError(OutOfMemory) => ErrorKind::OutOfVideoMemory(by.to_owned()),
                 e @ TextureCreationError(FormatNotSupported) |
                 e @ TextureCreationError(NotSupported) |
                 e @ TextureCreationError(TooLarge) |
                 e @ BufferCreationError(BufferTypeNotSupported) => {
                     ErrorKind::UnsupportedFeature(e.to_string(), by.to_owned())
                 }
             }).into()
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::ProgramCreationError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        use glium::ProgramCreationError::*;
        self.map_err(|e| {
            (match e {
                 CompilationError(log) |
                 LinkingError(log) => ErrorKind::Shader(log, by.to_owned()),
                 BinaryHeaderError => {
                     ErrorKind::Shader("Binary header error.".to_owned(), by.to_owned())
                 }

                 e @ ShaderTypeNotSupported |
                 e @ CompilationNotSupported |
                 e @ TransformFeedbackNotSupported |
                 e @ PointSizeNotSupported => {
                     ErrorKind::UnsupportedFeature(e.to_string(), by.to_owned())
                 }
             }).into()
        })
    }
}

impl<S> NeededBy for StdResult<S, glium::DrawError> {
    type Success = S;

    fn needed_by(self, by: &str) -> Result<Self::Success> {
        use glium::DrawError::*;
        self.map_err(|e| match e {
            e @ FixedIndexRestartingNotSupported |
            e @ ViewportTooLarge |
            e @ UnsupportedVerticesPerPatch |
            e @ TessellationNotSupported |
            e @ SamplersNotSupported |
            e @ TransformFeedbackNotSupported |
            e @ SmoothingNotSupported |
            e @ ProvokingVertexNotSupported |
            e @ RasterizerDiscardNotSupported |
            e @ DepthClampNotSupported |
            e @ BlendingParameterNotSupported => {
                ErrorKind::UnsupportedFeature(e.to_string(), by.to_owned()).into()
            }

            e @ NoDepthBuffer |
            e @ AttributeTypeMismatch |
            e @ AttributeMissing |
            e @ InvalidDepthRange |
            e @ UniformTypeMismatch { .. } |
            e @ UniformBufferToValue { .. } |
            e @ UniformValueToBlock { .. } |
            e @ UniformBlockLayoutMismatch { .. } |
            e @ TessellationWithoutPatches |
            e @ InstancesCountMismatch |
            e @ VerticesSourcesLengthMismatch |
            e @ SubroutineNotFound { .. } |
            e @ SubroutineUniformMissing { .. } |
            e @ SubroutineUniformToValue { .. } |
            e @ WrongQueryOperation => panic!("Invalid draw call: {:?}", e),
        })
    }
}

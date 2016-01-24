use std::fmt::{Display, Formatter, Result as FmtResult};
use std::marker::PhantomData;
use std::error::Error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GliumCreationError<T> {
    BackendCreationError(T),
    IncompatibleOpenGl(String),
}

impl<T: Error> Display for GliumCreationError<T> where T: Error
{
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "glium-creation-error")
    }
}

impl<T: Error> Error for GliumCreationError<T> where T: Error
{
    fn description(&self) -> &str {
        "glium-creation-error"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

pub mod uniforms {
    pub struct UniformStorage;

    pub trait AsUniformValue {
        fn as_uniform_value(&self) -> UniformValue;
    }

    #[derive(Debug,Copy, Clone)]
    pub enum UniformValue<'a> {
        Texture2d(&'a super::texture::Texture2d, Option<SamplerBehavior>),
        Float(f32),
        Vec2([f32; 2]),
        Mat4([[f32; 4]; 4]),
    }

    pub trait Uniforms {
        fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, F);
    }

    impl Uniforms for UniformStorage {
        fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, _f: F) {}
    }

    #[derive(Copy, Clone, Debug)]
    pub struct SamplerBehavior {
        pub wrap_function: (SamplerWrapFunction,
                            SamplerWrapFunction,
                            SamplerWrapFunction),
        pub minify_filter: MinifySamplerFilter,
        pub magnify_filter: MagnifySamplerFilter,
        pub max_anisotropy: u16,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum SamplerWrapFunction {
        Repeat,
        Mirror,
        Clamp,
        MirrorClamp,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum MinifySamplerFilter {
        Nearest,
        Linear,
        NearestMipmapNearest,
        LinearMipmapNearest,
        NearestMipmapLinear,
        LinearMipmapLinear,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum MagnifySamplerFilter {
        Nearest,
        Linear,
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BackfaceCullingMode {
    CullClockwise,
}
impl Default for BackfaceCullingMode {
    fn default() -> Self {
        BackfaceCullingMode::CullClockwise
    }
}

pub struct VertexBuffer<T: Copy>(PhantomData<T>);

impl<T: Copy> VertexBuffer<T> {
    pub fn immutable<F>(_facade: &F,
                        _data: &[T])
                        -> Result<VertexBuffer<T>, vertex::BufferCreationError> {
        Ok(VertexBuffer(PhantomData))
    }
}

#[derive(Default)]
pub struct Depth {
    pub test: DepthTest,
    pub write: bool,
}

pub enum DepthTest {
    IfLess,
}
impl Default for DepthTest {
    fn default() -> Self {
        DepthTest::IfLess
    }
}

#[derive(Default)]
pub struct DrawParameters<'a> {
    pub depth: Depth,
    pub blend: Blend,
    pub backface_culling: BackfaceCullingMode,
    pub _phantom: PhantomData<&'a ()>,
}

#[derive(Default)]
pub struct Blend;
impl Blend {
    pub fn alpha_blending() -> Blend {
        Blend
    }
}

pub struct Frame;
impl Frame {
    pub fn finish(self) -> Result<(), ()> {
        Ok(())
    }
}
impl Surface for Frame {}

pub struct Program;

impl Program {
    pub fn new<F>(_facade: &F,
                  _input: program::ProgramCreationInput)
                  -> Result<Program, ProgramCreationError> {
        Ok(Program)
    }

    pub fn from_source<F>(_facade: &F,
                          _vertex_src: &str,
                          _frag_src: &str,
                          _geom_src: Option<&str>)
                          -> Result<Program, ProgramCreationError> {
        Ok(Program)
    }
}

pub mod texture {
    use std::borrow::Cow;
    use std::fmt::{Display, Formatter, Result as FmtResult};

    #[derive(Debug)]
    pub struct Texture2d {
        width: u32,
        height: u32,
    }

    impl Texture2d {
        pub fn new<'a, F, T: Clone>(_facade: &F,
                                    image: RawImage2d<'a, T>)
                                    -> Result<Texture2d, TextureCreationError> {
            Ok(Texture2d {
                width: image.width,
                height: image.height,
            })
        }

        pub fn empty<F>(_facade: &F,
                        width: u32,
                        height: u32)
                        -> Result<Texture2d, TextureCreationError> {
            Ok(Texture2d {
                width: width,
                height: height,
            })
        }

        pub fn get_width(&self) -> u32 {
            self.width
        }

        pub fn get_height(&self) -> Option<u32> {
            Some(self.height)
        }
    }

    pub enum ClientFormat {
        U8U8U8U8,
        U8U8U8,
        U8U8,
        U8,
    }

    pub struct RawImage2d<'a, T: Clone + 'a> {
        pub data: Cow<'a, [T]>,
        pub width: u32,
        pub height: u32,
        pub format: ClientFormat,
    }

    #[derive(Debug)]
    pub enum TextureCreationError {
        FormatNotSupported,
        DimensionsNotSupported,
        TypeNotSupported,
    }

    impl Display for TextureCreationError {
        fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
            write!(fmt, "{:?}", self)
        }
    }

    pub mod buffer_texture {
        use std::fmt::{Display, Formatter, Result as FmtResult};
        use super::super::buffer::Mapping;
        pub struct BufferTexture<T: Default + Clone>(Vec<T>);

        impl<T: Default + Clone> BufferTexture<T> {
            pub fn empty_persistent<F>(_facade: &F,
                                       size: usize,
                                       _type: BufferTextureType)
                                       -> Result<BufferTexture<T>, CreationError> {
                Ok(BufferTexture(vec![T::default(); size]))
            }

            pub fn map(&mut self) -> Mapping<[T]> {
                Mapping(&mut self.0[..])
            }
        }


        impl<T: Clone + Default> super::super::uniforms::AsUniformValue for BufferTexture<T> {
            fn as_uniform_value(&self) -> super::super::uniforms::UniformValue {
                super::super::uniforms::UniformValue::Float(0.0)
            }
        }

        pub enum BufferTextureType {
            Float,
        }

        #[derive(Debug)]
        pub enum TextureCreationError {
            NotSupported,
            FormatNotSupported,
            TooLarge,
        }
        impl Display for TextureCreationError {
            fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
                write!(fmt, "{:?}", self)
            }
        }


        #[derive(Debug)]
        pub enum CreationError {
            BufferCreationError(super::super::buffer::BufferCreationError),
            TextureCreationError(TextureCreationError),
        }
        impl Display for CreationError {
            fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
                write!(fmt, "{:?}", self)
            }
        }

    }
}

pub mod buffer {
    use std::fmt::{Display, Formatter, Result as FmtResult};
    use std::ops::{Deref, DerefMut};

    #[derive(Debug)]
    pub enum BufferCreationError {
        OutOfMemory,
        BufferTypeNotSupported,
    }

    impl Display for BufferCreationError {
        fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
            write!(fmt, "{:?}", self)
        }
    }

    pub struct Mapping<'b, D: ?Sized + 'b>(pub &'b mut D);
    impl<'a, D: ?Sized> Deref for Mapping<'a, D> {
        type Target = D;

        fn deref(&self) -> &D {
            self.0
        }
    }

    impl<'a, D: ?Sized> DerefMut for Mapping<'a, D> {
        fn deref_mut(&mut self) -> &mut D {
            self.0
        }
    }
}

pub mod vertex {
    use std::fmt::{Display, Formatter, Result as FmtResult};

    #[derive(Debug)]
    pub enum BufferCreationError {
        FormatNotSupported,
        BufferCreationError(super::buffer::BufferCreationError),
    }

    impl Display for BufferCreationError {
        fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
            write!(fmt, "{:?}", self)
        }
    }
}

pub mod program {
    pub enum TransformFeedbackMode {
        Interleaved,
        Separate,
    }

    pub enum ProgramCreationInput<'a> {
        SourceCode {
            vertex_shader: &'a str,
            tessellation_control_shader: Option<&'a str>,
            tessellation_evaluation_shader: Option<&'a str>,
            geometry_shader: Option<&'a str>,
            fragment_shader: &'a str,
            transform_feedback_varyings: Option<(Vec<String>, TransformFeedbackMode)>,
            outputs_srgb: bool,
            uses_point_size: bool,
        },
    }
}

#[derive(Debug)]
pub enum ProgramCreationError {
    CompilationError(String),
    LinkingError(String),
    ShaderTypeNotSupported,
    CompilationNotSupported,
    TransformFeedbackNotSupported,
    PointSizeNotSupported,
}
impl Display for ProgramCreationError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum DrawError {
    NoDepthBuffer,
    AttributeTypeMismatch,
    AttributeMissing,
    ViewportTooLarge,
    InvalidDepthRange,
    UniformTypeMismatch {
        _dummy: (),
    },
    UniformBufferToValue {
        _dummy: (),
    },
    UniformValueToBlock {
        _dummy: (),
    },
    UniformBlockLayoutMismatch {
        _dummy: (),
    },
    UnsupportedVerticesPerPatch,
    TessellationNotSupported,
    TessellationWithoutPatches,
    SamplersNotSupported,
    InstancesCountMismatch,
    VerticesSourcesLengthMismatch,
    TransformFeedbackNotSupported,
    WrongQueryOperation,
    SmoothingNotSupported,
    ProvokingVertexNotSupported,
    RasterizerDiscardNotSupported,
    DepthClampNotSupported,
    BlendingParameterNotSupported,
}

pub trait Surface: Sized {
    fn draw<'a, 'b, T: Copy, U>(&mut self,
                                _v: &VertexBuffer<T>,
                                _i: index::NoIndices,
                                _p: &Program,
                                _u: &U,
                                _d: &DrawParameters)
                                -> Result<(), DrawError>
        where U: uniforms::Uniforms
    {
        Ok(())
    }

    fn clear_all_srgb(&mut self, _c: (f32, f32, f32, f32), _d: f32, _s: i32) {}
}

pub mod index {
    pub struct NoIndices(pub PrimitiveType);
    pub enum PrimitiveType {
        TrianglesList,
        TriangleStrip,
    }
}


#[macro_export]
macro_rules! implement_vertex {
    ($struct_name:ident, $($field_name:ident),+) => {};
    ($struct_name:ident, $($field_name:ident),+,) => {};
}

#[macro_export]
macro_rules! uniform {
    () => { $crate::uniforms::UniformStorage };
    ($field:ident: $value:expr) => { $crate::uniforms::UniformStorage };
    ($field1:ident: $value1:expr, $($field:ident: $value:expr),+) => {
        $crate::uniforms::UniformStorage
    };
    ($($field:ident: $value:expr),*,) => { $crate::uniforms::UniformStorage };
}

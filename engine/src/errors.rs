use failchain::{BoxedError, ChainErrorKind};
use failure::Fail;
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

pub(crate) trait UnsupportedFeature: Fail + Sized {}

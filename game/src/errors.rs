use failchain::{ChainErrorKind, UnboxedError};
use failure::Fail;
use std::result::Result as StdResult;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
#[fail(display = "Game error: {}", 0)]
pub struct ErrorKind(pub(crate) String);

pub type Error = UnboxedError<ErrorKind>;
pub type Result<T> = StdResult<T, Error>;

impl ChainErrorKind for ErrorKind {
    type Error = Error;
}

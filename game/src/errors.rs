use engine;
use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::result::Result as StdResult;
use wad;

#[derive(Debug)]
pub struct Error {
    inner: Context<String>,
}

pub trait ResultExt {
    type Success;
    type Error;

    fn err_context<StringT: Into<String>>(
        self,
        mapper: impl FnOnce(&Self::Error) -> StringT,
    ) -> Result<Self::Success>;
}

impl<SuccessT, ErrorT: Fail> ResultExt for StdResult<SuccessT, ErrorT> {
    type Success = SuccessT;
    type Error = ErrorT;

    fn err_context<StringT: Into<String>>(
        self,
        mapper: impl FnOnce(&ErrorT) -> StringT,
    ) -> Result<SuccessT> {
        self.map_err(|error| {
            let context = mapper(&error).into();
            Error::from(error.context(context))
        })
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
        write!(f, "Game error {}", self.inner.get_context())
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::from(Context::new(message))
    }
}

impl From<engine::Error> for Error {
    fn from(error: engine::Error) -> Self {
        Self::from(error.context("caused by engine".to_owned()))
    }
}

impl From<wad::Error> for Error {
    fn from(error: wad::Error) -> Self {
        Self::from(error.context("caused by wad".to_owned()))
    }
}

impl From<Context<String>> for Error {
    fn from(inner: Context<String>) -> Self {
        Error { inner }
    }
}

pub type Result<T> = StdResult<T, Error>;

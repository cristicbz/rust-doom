use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;

#[derive(Debug)]
pub struct GeneralError(pub String);

impl Error for GeneralError {
    fn description(&self) -> &str {
        &self.0[..]
    }
}

impl From<String> for GeneralError {
    fn from(message: String) -> GeneralError {
        GeneralError(message)
    }
}

impl<'a> From<&'a str> for GeneralError {
    fn from(message: &'a str) -> GeneralError {
        GeneralError(message.to_owned())
    }
}

impl Display for GeneralError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{}", self.0)
    }
}

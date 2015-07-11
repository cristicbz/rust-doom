use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;
use std::io::{self, Read};
use std::mem;
use std::slice;

pub trait ReadExt: Read {
    fn read_at_least(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        if buf.len() == 0 { return Ok(()); }
        let len = try!(self.read(buf));
        self.read_at_least(&mut buf[len..])
    }

    fn read_binary<T: Copy>(&mut self) -> io::Result<T> {
        let mut loaded = unsafe { mem::uninitialized::<T>() };
        let size = mem::size_of::<T>();
        try!(self.read_at_least(unsafe {
            slice::from_raw_parts_mut(&mut loaded as *mut _ as *mut u8, size)
        }));
        Ok(loaded)
    }
}

impl<T: Read> ReadExt for T {}

#[derive(Debug)]
pub struct GeneralError(pub String);

impl Error for GeneralError {
    fn description(&self) -> &str { &self.0[..] }
}

impl From<String> for GeneralError {
    fn from(message: String) -> GeneralError { GeneralError(message) }
}

impl<'a> From<&'a str> for GeneralError {
    fn from(message: &'a str) -> GeneralError { GeneralError(message.to_owned()) }
}

impl Display for GeneralError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult { write!(fmt, "{}", self.0) }
}


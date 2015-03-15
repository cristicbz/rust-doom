#![feature(collections, std_misc, core)]

use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::mem;
use std::path::AsPath;
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

impl<R: Read> ReadExt for R {}

pub fn read_utf8_file<P: AsPath>(path: &P) -> Result<String, String> {
    File::open(path)
        .and_then(|mut file| {
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).map(|_| buffer)
        })
        .map_err(|e| String::from_str(Error::description(&e)))
        .and_then(|buffer| {
            String::from_utf8(buffer).map_err(|_| {
                format!("File at '{:?}' is not valid UTF-8.", path.as_path())
            })
        })
}


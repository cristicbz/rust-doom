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

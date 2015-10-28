use byteorder::{LittleEndian, ByteOrder, ReadBytesExt};
use error::Result;
use std::io::Read;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::io::Result as IoResult;


pub trait WadReadFrom: Sized {
    fn wad_read_from<R: Read>(&mut R) -> Result<Self>;

    #[inline]
    fn wad_read_many_from<R: Read>(reader: &mut R, n: usize) -> Result<Vec<Self>> {
        (0..n).map(|_| reader.wad_read()).collect()
    }
}

impl WadReadFrom for u8 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_u8()))
    }

    #[inline]
    fn wad_read_many_from<R: Read>(reader: &mut R, n: usize) -> Result<Vec<Self>> {
        let mut buf = vec![0; n];
        try!(reader.read_buffer(&mut buf));
        Ok(buf)
    }
}

impl WadReadFrom for u16 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_u16::<LittleEndian>()))
    }
}

impl WadReadFrom for u32 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_u32::<LittleEndian>()))
    }
}

impl WadReadFrom for u64 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_u64::<LittleEndian>()))
    }
}

impl WadReadFrom for i8 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_i8()))
    }
}

impl WadReadFrom for i16 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_i16::<LittleEndian>()))
    }
}

impl WadReadFrom for i32 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_i32::<LittleEndian>()))
    }
}

impl WadReadFrom for i64 {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(try!(reader.read_i64::<LittleEndian>()))
    }
}

impl WadReadFrom for [u8; 256] {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut ret = [0; 256];
        try!(reader.read_buffer(&mut ret));
        Ok(ret)
    }
}

impl WadReadFrom for [u8; 768] {
    #[inline]
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut ret = [0; 768];
        try!(reader.read_buffer(&mut ret));
        Ok(ret)
    }
}

pub trait WadRead: Read + Sized {
    #[inline]
    fn wad_read<T: WadReadFrom>(&mut self) -> Result<T> {
        T::wad_read_from(self)
    }

    #[inline]
    fn wad_read_many<T: WadReadFrom>(&mut self, n: usize) -> Result<Vec<T>> {
        T::wad_read_many_from(self, n)
    }

    fn read_buffer(&mut self, mut buf: &mut [u8]) -> IoResult<()> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                Err(ref e) if e.kind() == IoErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(IoError::new(IoErrorKind::InvalidData, "failed to fill whole buffer"))
        } else {
            Ok(())
        }
    }
}

impl<T: Read> WadRead for T {}

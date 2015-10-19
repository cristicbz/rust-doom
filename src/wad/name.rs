use error::{Result, ErrorKind};
use std::result::Result as StdResult;
use read::{WadReadFrom, WadRead};
use rustc_serialize::{Encoder, Encodable, Decoder, Decodable};
use std::ascii::AsciiExt;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::io::Read;
use std::str;
use std::error::Error;
use std::ops::Deref;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct WadName([u8; 8]);

impl Deref for WadName {
    type Target = str;
    fn deref(&self) -> &str {
        str::from_utf8(&self.0).unwrap()  // Guaranteed by construction invariants.
    }
}

impl WadName {
    pub fn from_bytes(value: &[u8]) -> Result<WadName> {
        let mut name = [0u8; 8];
        let mut nulled = false;
        for (dest, src) in name.iter_mut().zip(value.iter()) {
            if !src.is_ascii() {
                debug!("Bailed on non-ascii {}", src);
                return Err(ErrorKind::BadWadName(value.iter().cloned().collect()).into());
            }

            let new_byte = match src.to_ascii_uppercase() {
                b@b'A'...b'Z' |
                b@b'0'...b'9' |
                b@b'_' |
                b@b'-' |
                b@b'[' |
                b@b']' |
                b@b'\\' => b,
                b'\0' => { nulled = true; break },
                b => {
                    debug!("Bailed on ascii {}", b);
                    return Err(ErrorKind::BadWadName(value.iter().cloned().collect()).into());
                }
            };
            *dest = new_byte;
        }
        if !nulled && value.len() > 8 {
            debug!("Bailed on '{:?}' {} {}", str::from_utf8(value), value.len(), !nulled);
            Err(ErrorKind::BadWadName(value.iter().cloned().collect()).into())
        } else {
            Ok(WadName(name))
        }
    }

    pub fn from_str(value: &str) -> Result<WadName> {
        WadName::from_bytes(value.as_bytes())
    }
}

impl Display for WadName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.deref())
    }
}
impl Debug for WadName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "WadName({:?})", self.deref())
    }
}
impl Encodable for WadName {
    fn encode<S: Encoder>(&self, encoder: &mut S) -> StdResult<(), <S as Encoder>::Error> {
        self.deref().encode(encoder)
    }
}

impl Decodable for WadName {
    fn decode<S: Decoder>(decoder: &mut S)
            -> StdResult<WadName, <S as Decoder>::Error> {
        decoder.read_str()
               .and_then(|s| {
                   WadName::from_str(&s).map_err(|_| decoder.error("Could not decode WadName."))
               })
    }
}

impl WadReadFrom for WadName {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let bytes = try!(reader.wad_read::<u64>());
        WadName::from_bytes(&[((bytes >>  0) & 0xff) as u8, ((bytes >>  8) & 0xff) as u8,
                              ((bytes >> 16) & 0xff) as u8, ((bytes >> 24) & 0xff) as u8,
                              ((bytes >> 32) & 0xff) as u8, ((bytes >> 40) & 0xff) as u8,
                              ((bytes >> 48) & 0xff) as u8, ((bytes >> 56) & 0xff) as u8])
    }
}

impl PartialEq<str> for WadName {
    fn eq(&self, rhs: &str) -> bool {
        self.deref() == rhs
    }
}

#[cfg(test)]
mod test {
    use super::WadName;

    #[test]
    fn test_wad_name() {
        assert_eq!(&WadName::from_str("").unwrap(), "\0\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("\0").unwrap(), "\0\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("\01234567").unwrap(), "\0\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("A").unwrap(), "A\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("1234567").unwrap(), "1234567\0");
        assert_eq!(&WadName::from_str("12345678").unwrap(), "12345678");
        assert_eq!(&WadName::from_str("123\05678").unwrap(), "123\0\0\0\0\0");
        assert_eq!(&WadName::from_str("SKY1").unwrap(), "SKY1\0\0\0\0");
        assert_eq!(&WadName::from_str("-").unwrap(), "-\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("_").unwrap(), "_\0\0\0\0\0\0\0");

        assert!(WadName::from_bytes(b"123456789").is_err());
        assert!(WadName::from_bytes(b"1234\xfb").is_err());
        assert!(WadName::from_bytes(b"\xff123").is_err());
        assert!(WadName::from_bytes(b"$$ASDF_").is_err());
        assert!(WadName::from_bytes(b"123456789\0").is_err());
    }
}

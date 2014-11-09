use std::{fmt, mem, str};
use std::fmt::Show;
use std::string::String;
use serialize::{Encoder, Encodable, Decoder, Decodable};

#[deriving(Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct WadName { packed: u64 }
impl WadName {
    pub fn as_str_opt(&self) -> Option<&str> {
        str::from_utf8(self.as_bytes())
    }

    pub fn as_str(&self) -> &str {
        match self.as_str_opt() {
            Some(s) => s,
            None => panic!("Failed WadName.as_str(): {}", self)
        }
    }

    pub fn as_bytes(&self) -> &[u8, ..8] {
        unsafe { mem::transmute::<_, &[u8, ..8]>(&self.packed) }
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8, ..8] {
        unsafe { mem::transmute::<_, &mut [u8, ..8]>(&mut self.packed) }
    }

    pub fn into_canonical(mut self) -> WadName {
        self.canonicalise();
        self
    }

    pub fn canonicalise(&mut self) -> &mut WadName {
        let new_packed = match self.as_bytes().to_wad_name_opt() {
            Some(name) => name.packed,
            None => panic!("Malformed wad name: {}", self)
        };
        self.packed = new_packed;
        self
    }
}
impl Show for WadName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.as_str_opt() {
            Some(s) => write!(formatter, "{}", s),
            None => write!(formatter, "BadWadName({} {} {} {} {} {} {} {})",
                           self.as_bytes()[0], self.as_bytes()[1],
                           self.as_bytes()[2], self.as_bytes()[3],
                           self.as_bytes()[4], self.as_bytes()[5],
                           self.as_bytes()[6], self.as_bytes()[7])
        }
    }
}
impl<S: Encoder<E>, E> Encodable<S, E> for WadName {
    fn encode(&self, encoder: &mut S) -> Result<(), E> {
        match self.as_str_opt() {
            Some(s) => s.encode(encoder),
            None => panic!("Cannot encode WadName {}", self)
        }
    }
}

impl<S: Decoder<E>, E> Decodable<S, E> for WadName {
    fn decode(decoder: &mut S) -> Result<WadName, E> {
        decoder.read_str()
            .and_then(|s| {
                match s.to_wad_name_opt() {
                    Some(name) => Ok(name),
                    None => Err(decoder.error("Could not decode WadName."))
                }
            })
    }
}

pub trait WadNameCast : Show {
    fn to_wad_name_opt(&self) -> Option<WadName>;
    fn to_wad_name(&self) -> WadName {
        match self.to_wad_name_opt() {
            Some(n) => n,
            None => panic!("Malformed WadName cast {}", self)
        }
    }
}
impl<'a> WadNameCast for &'a [u8] {
    fn to_wad_name_opt(&self) -> Option<WadName> {
        let mut name = [0u8, ..8];
        let mut nulled = false;
        for (dest, src) in name.iter_mut().zip(self.iter()) {
            let new_byte = match src.to_ascii_opt() {
                Some(ascii) => match ascii.to_uppercase().to_byte() {
                    b@b'A'...b'Z' | b@b'0'...b'9' | b@b'_' | b@b'-' |
                    b@b'[' | b@b']' | b@b'\\' => b,
                    b'\0' => { nulled = true; break },
                    b => {
                        debug!("Bailed on ascii {}", b);
                        return None;
                    }
                },
                None => {
                    debug!("Bailed on non-ascii {}", src);
                    return None;
                }
            };
            *dest = new_byte;
        }
        if !nulled && self.len() > 8 {
            debug!("Bailed on '{}' {} {}", str::from_utf8(*self),
                                           self.len(), !nulled);
            return None; }
        Some(WadName { packed: unsafe { mem::transmute(name) } })
    }
}
impl<'a> WadNameCast for &'a str {
    fn to_wad_name_opt(&self) -> Option<WadName> {
        self.as_bytes().to_wad_name_opt()
    }
}
impl WadNameCast for String {
    fn to_wad_name_opt(&self) -> Option<WadName> {
        self.as_bytes().to_wad_name_opt()
    }
}


#[cfg(test)]
mod test {
    use super::WadNameCast;

    #[test]
    fn test_wad_name() {
        assert_eq!("".to_wad_name().as_str(), "\0\0\0\0\0\0\0\0");
        assert_eq!("\0".to_wad_name().as_str(), "\0\0\0\0\0\0\0\0");
        assert_eq!("\01234567".to_wad_name().as_str(), "\0\0\0\0\0\0\0\0");
        assert_eq!("A".to_wad_name().as_str(), "A\0\0\0\0\0\0\0");
        assert_eq!("1234567".to_wad_name().as_str(), "1234567\0");
        assert_eq!("12345678".to_wad_name().as_str(), "12345678");
        assert_eq!("123\05678".to_wad_name().as_str(), "123\0\0\0\0\0");
        assert_eq!("SKY1".to_wad_name().as_str(), "SKY1\0\0\0\0");
        assert_eq!("-".to_wad_name().as_str(), "-\0\0\0\0\0\0\0");
        assert_eq!("_".to_wad_name().as_str(), "_\0\0\0\0\0\0\0");

        assert!(b"123456789".to_wad_name_opt().is_none());
        assert!(b"1234\xfb".to_wad_name_opt().is_none());
        assert!(b"\xff123".to_wad_name_opt().is_none());
        assert!(b"$$ASDF_".to_wad_name_opt().is_none());
        assert!(b"123456789\0".to_wad_name_opt().is_none());
    }
}


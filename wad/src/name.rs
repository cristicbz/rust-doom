use super::errors::{ErrorKind, Result};
use failchain::{bail, ensure};
use serde::de::{Deserialize, Deserializer, Error as SerdeDeError};
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::ops::Deref;
use std::result::Result as StdResult;
use std::str::{self, FromStr};

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct WadName([u8; 8]);

impl WadName {
    pub fn push(&mut self, new_byte: u8) -> Result<()> {
        let new_byte = match new_byte.to_ascii_uppercase() {
            b @ b'A'..=b'Z'
            | b @ b'0'..=b'9'
            | b @ b'_'
            | b @ b'%'
            | b @ b'-'
            | b @ b'['
            | b @ b']'
            | b @ b'\\' => b,
            b => {
                bail!(ErrorKind::invalid_byte_in_wad_name(b, &self.0));
            }
        };

        for byte in &mut self.0 {
            if *byte == 0 {
                *byte = new_byte;
                return Ok(());
            }
        }

        bail!(ErrorKind::wad_name_too_long(&self.0));
    }

    pub fn from_bytes(value: &[u8]) -> Result<WadName> {
        let mut name = [0u8; 8];
        let mut nulled = false;
        for (dest, &src) in name.iter_mut().zip(value.iter()) {
            ensure!(
                src.is_ascii(),
                ErrorKind::invalid_byte_in_wad_name(src, value)
            );

            let new_byte = match src.to_ascii_uppercase() {
                b @ b'A'..=b'Z'
                | b @ b'0'..=b'9'
                | b @ b'_'
                | b @ b'-'
                | b @ b'['
                | b @ b']'
                | b @ b'%'
                | b @ b'\\' => b,
                b'\0' => {
                    nulled = true;
                    break;
                }
                b => {
                    bail!(ErrorKind::invalid_byte_in_wad_name(b, value));
                }
            };
            *dest = new_byte;
        }

        ensure!(
            nulled || value.len() <= 8,
            ErrorKind::wad_name_too_long(value)
        );
        Ok(WadName(name))
    }
}

impl FromStr for WadName {
    type Err = super::errors::Error;
    fn from_str(value: &str) -> Result<WadName> {
        WadName::from_bytes(value.as_bytes())
    }
}

impl Display for WadName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", str::from_utf8(&self[..]).unwrap())
    }
}

impl Deref for WadName {
    type Target = [u8; 8];
    fn deref(&self) -> &[u8; 8] {
        &self.0
    }
}

impl Debug for WadName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "WadName({:?})",
            str::from_utf8(&self[..]).unwrap()
        )
    }
}

impl PartialEq<[u8; 8]> for WadName {
    fn eq(&self, rhs: &[u8; 8]) -> bool {
        self.deref() == rhs
    }
}

impl Borrow<[u8; 8]> for WadName {
    fn borrow(&self) -> &[u8; 8] {
        self.deref()
    }
}

impl AsRef<str> for WadName {
    fn as_ref(&self) -> &str {
        str::from_utf8(self.deref()).expect("wad name is not valid utf-8")
    }
}

impl<'de> Deserialize<'de> for WadName {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        WadName::from_bytes(&<[u8; 8]>::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

pub trait IntoWadName {
    fn into_wad_name(self) -> Result<WadName>;
}

impl IntoWadName for &[u8] {
    fn into_wad_name(self) -> Result<WadName> {
        WadName::from_bytes(self)
    }
}

impl IntoWadName for &[u8; 8] {
    fn into_wad_name(self) -> Result<WadName> {
        WadName::from_bytes(self)
    }
}

impl IntoWadName for &str {
    fn into_wad_name(self) -> Result<WadName> {
        WadName::from_str(self)
    }
}

impl IntoWadName for WadName {
    fn into_wad_name(self) -> Result<WadName> {
        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::WadName;
    use std::str::FromStr;

    #[test]
    fn test_wad_name() {
        assert_eq!(&WadName::from_str("").unwrap(), b"\0\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("\0").unwrap(), b"\0\0\0\0\0\0\0\0");
        assert_eq!(
            &WadName::from_str("\01234567").unwrap(),
            b"\0\0\0\0\0\0\0\0"
        );
        assert_eq!(&WadName::from_str("A").unwrap(), b"A\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("1234567").unwrap(), b"1234567\0");
        assert_eq!(&WadName::from_str("12345678").unwrap(), b"12345678");
        assert_eq!(&WadName::from_str("123\05678").unwrap(), b"123\0\0\0\0\0");
        assert_eq!(&WadName::from_str("SKY1").unwrap(), b"SKY1\0\0\0\0");
        assert_eq!(&WadName::from_str("-").unwrap(), b"-\0\0\0\0\0\0\0");
        assert_eq!(&WadName::from_str("_").unwrap(), b"_\0\0\0\0\0\0\0");

        assert!(WadName::from_bytes(b"123456789").is_err());
        assert!(WadName::from_bytes(b"1234\xfb").is_err());
        assert!(WadName::from_bytes(b"\xff123").is_err());
        assert!(WadName::from_bytes(b"$$ASDF_").is_err());
        assert!(WadName::from_bytes(b"123456789\0").is_err());
    }
}

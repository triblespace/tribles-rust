use std::convert::TryFrom;

use crate::{Value, ValueParseError, Valuelike};

#[derive(Debug, Clone)]
pub enum FromStrError {
    TooLong,
    InteriorNul,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct ShortString(Value);

impl ShortString {
    pub fn new<S: AsRef<str>>(s: S) -> Result<ShortString, FromStrError> {
        let str_ref: &str = s.as_ref();
        let bytes = str_ref.as_bytes();
        if bytes.len() > 32 {
            return Err(FromStrError::TooLong);
        }
        if bytes.iter().any(|&b| b == 0) {
            return Err(FromStrError::InteriorNul);
        }

        let mut data: [u8; 32] = [0; 32];
        data[..bytes.len()].copy_from_slice(bytes);

        Ok(ShortString(data))
    }
}

impl Valuelike for ShortString {
    fn from_value(bytes: Value) -> Result<Self, ValueParseError> {
        std::str::from_utf8(&bytes[..])
            .map_err(|_| ValueParseError::new(bytes, "failed to convert to utf-8 string"))?;
        Ok(ShortString(bytes))
    }

    fn into_value(shortstring: &Self) -> Value {
        shortstring.0
    }
}

impl From<&ShortString> for String {
    fn from(s: &ShortString) -> Self {
        unsafe {
            String::from_utf8_unchecked(
                s.0[0..s.0.iter().position(|&b| b == 0).unwrap_or(s.0.len())].into(),
            )
        }
    }
}

impl<'a> From<&'a ShortString> for &'a str {
    fn from(s: &'a ShortString) -> Self {
        unsafe {
            std::str::from_utf8_unchecked(
                &s.0[0..s.0.iter().position(|&b| b == 0).unwrap_or(s.0.len())],
            )
        }
    }
}

impl TryFrom<&str> for ShortString {
    type Error = FromStrError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        ShortString::new(s)
    }
}

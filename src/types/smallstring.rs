use std::convert::TryFrom;

use crate::{Value, ValueParseError, Valuelike};

#[derive(Debug, Clone)]
pub enum FromStrError {
    TooLong,
    InteriorNul,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct SmallString(Value);

impl SmallString {
    pub fn new<S: AsRef<str>>(s: S) -> Result<SmallString, FromStrError> {
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

        Ok(SmallString(data))
    }
}

impl Valuelike for SmallString {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        std::str::from_utf8(&value[..])
            .map_err(|_| ValueParseError::new(value, "failed to convert to utf-8 string"))?;
        Ok(SmallString(value))
    }

    fn into_value(value: &Self) -> Value {
        value.0
    }
}

impl From<&SmallString> for String {
    fn from(s: &SmallString) -> Self {
        unsafe {
            String::from_utf8_unchecked(
                s.0[0..s.0.iter().position(|&b| b == 0).unwrap_or(s.0.len())].into(),
            )
        }
    }
}

impl<'a> From<&'a SmallString> for &'a str {
    fn from(s: &'a SmallString) -> Self {
        unsafe {
            std::str::from_utf8_unchecked(
                &s.0[0..s.0.iter().position(|&b| b == 0).unwrap_or(s.0.len())],
            )
        }
    }
}

impl TryFrom<&str> for SmallString {
    type Error = FromStrError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        SmallString::new(s)
    }
}

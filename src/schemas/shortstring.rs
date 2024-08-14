use std::{convert::TryFrom, str::Utf8Error, string::FromUtf8Error};

use crate::Value;

#[derive(Debug, Clone)]
pub enum FromStrError {
    TooLong,
    InteriorNul,
}

pub struct ShortString;

impl TryFrom<&Value<ShortString>> for String {
    type Error = FromUtf8Error;
    
    fn try_from(value: &Value<ShortString>) -> Result<Self, Self::Error> {
        String::from_utf8(
            value.bytes[0..value.bytes.iter().position(|&b| b == 0).unwrap_or(value.bytes.len())].into(),
        )
    }
}

impl<'a> TryFrom<&'a Value<ShortString>> for &'a str {
    type Error = Utf8Error;
    
    fn try_from(value: &'a Value<ShortString>) -> Result<Self, Self::Error> {
        std::str::from_utf8(
            &value.bytes[0..value.bytes.iter().position(|&b| b == 0).unwrap_or(value.bytes.len())],
        )
    }
}

impl TryFrom<&str> for Value<ShortString> {
    type Error = FromStrError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        if bytes.len() > 32 {
            return Err(FromStrError::TooLong);
        }
        if bytes.iter().any(|&b| b == 0) {
            return Err(FromStrError::InteriorNul);
        }

        let mut data: [u8; 32] = [0; 32];
        data[..bytes.len()].copy_from_slice(bytes);

        Ok(Value::new(data))
    }
}

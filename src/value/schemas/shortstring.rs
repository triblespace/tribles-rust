use crate::id::RawId;
use crate::value::{ToValue, TryToValue, TryFromValue, Value, ValueSchema};

use hex_literal::hex;
use std::{str::Utf8Error, string::FromUtf8Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromStrError {
    TooLong,
    InteriorNul,
}

pub struct ShortString;

impl ValueSchema for ShortString {
    const ID: RawId = hex!("2D848DB0AF112DB226A6BF1A3640D019");
}

impl<'a> TryFromValue<'a, ShortString> for String {
    type Error = FromUtf8Error;

    fn try_from_value(v: &Value<ShortString>) -> Result<Self, Self::Error> {
        String::from_utf8(
            v.bytes[0..v
                .bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(v.bytes.len())]
                .into(),
        )
    }
}

impl<'a> TryFromValue<'a, ShortString> for &'a str {
    type Error = Utf8Error;

    fn try_from_value(v: &'a Value<ShortString>) -> Result<&'a str, Self::Error> {
        std::str::from_utf8(
            &v.bytes[0..v
                .bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(v.bytes.len())],
        )
    }
}

impl TryToValue<ShortString> for &str {
    type Error = FromStrError;

    fn try_to_value(self) -> Result<Value<ShortString>, Self::Error> {
        let bytes = self.as_bytes();
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

impl ToValue<ShortString> for &str {
    fn to_value(self) -> Value<ShortString> {
        let bytes = self.as_bytes();
        if bytes.len() > 32 {
            panic!();
        }
        if bytes.iter().any(|&b| b == 0) {
            panic!();
        }

        let mut data: [u8; 32] = [0; 32];
        data[..bytes.len()].copy_from_slice(bytes);

        Value::new(data)
    }
}

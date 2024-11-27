use crate::id::RawId;
use crate::value::{FromValue, ToValue, TryFromValue, TryToValue, Value, ValueSchema};

use hex_literal::hex;
use indxvec::Printing;
use std::str::Utf8Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromStrError {
    TooLong,
    InteriorNul,
}

pub struct ShortString;

impl ValueSchema for ShortString {
    const ID: RawId = hex!("2D848DB0AF112DB226A6BF1A3640D019");
}

impl<'a> TryFromValue<'a, ShortString> for &'a str {
    type Error = Utf8Error;

    fn try_from_value(v: &'a Value<ShortString>) -> Result<&'a str, Self::Error> {
        std::str::from_utf8(
            &v.raw.bytes[0..
                v.raw.bytes
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(v.raw.bytes.len())],
        )
    }
}

impl<'a> TryFromValue<'a, ShortString> for String {
    type Error = Utf8Error;

    fn try_from_value(v: &Value<ShortString>) -> Result<Self, Self::Error> {
        let s: &str = v.try_from_value()?;
        Ok(s.to_string())
    }
}

impl<'a> FromValue<'a, ShortString> for &'a str {
    fn from_value(v: &'a Value<ShortString>) -> Self {
        v.try_from_value().unwrap()
    }
}

impl<'a> FromValue<'a, ShortString> for String {
    fn from_value(v: &'a Value<ShortString>) -> Self {
        v.try_from_value().unwrap()
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

        Ok(Value::new(&data))
    }
}

impl TryToValue<ShortString> for String {
    type Error = FromStrError;

    fn try_to_value(self) -> Result<Value<ShortString>, Self::Error> {
        (&self[..]).try_to_value()
    }
}

impl ToValue<ShortString> for &str {
    fn to_value(self) -> Value<ShortString> {
        self.try_to_value().unwrap()
    }
}

impl ToValue<ShortString> for String {
    fn to_value(self) -> Value<ShortString> {
        self.try_to_value().unwrap()
    }
}

impl ToValue<ShortString> for &String {
    fn to_value(self) -> Value<ShortString> {
        self.to_str().try_to_value().unwrap()
    }
}

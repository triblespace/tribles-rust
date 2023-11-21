use std::convert::TryFrom;

use crate::{inline_value, trible::*};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct ShortString(String);

inline_value!(ShortString);

impl ShortString {
    pub fn new(s: String) -> Result<ShortString, &'static str> {
        if s.len() >= 32 {
            return Err("String too long.");
        }
        if s.as_bytes().iter().any(|&b| b == 0) {
            return Err("ShortString must not contain null.");
        }
        Ok(ShortString(s))
    }
}

impl From<&ShortString> for Value {
    fn from(string: &ShortString) -> Self {
        let mut data = [0; 32];
        let bytes = string.0.as_bytes();
        data[..bytes.len()].copy_from_slice(bytes);
        data
    }
}

impl From<Value> for ShortString {
    fn from(bytes: Value) -> Self {
        ShortString(
            String::from_utf8(
                IntoIterator::into_iter(bytes)
                    .take_while(|&x| x != 0)
                    .collect(),
            )
            .unwrap(),
        )
    }
}

impl TryFrom<&str> for ShortString {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.len() > 32 {
            return Err("string too long");
        }
        if s.as_bytes().iter().any(|&x| x == 0) {
            return Err("string may not contain null byte");
        }
        Ok(ShortString(s.to_string()))
    }
}

impl TryFrom<String> for ShortString {
    type Error = &'static str;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() > 32 {
            return Err("string too long");
        }
        if s.as_bytes().iter().any(|&x| x == 0) {
            return Err("string may not contain null byte");
        }
        Ok(ShortString(s.to_string()))
    }
}

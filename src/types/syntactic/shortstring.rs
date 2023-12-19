use std::convert::TryFrom;

use crate::types::{Value, Valuelike};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct ShortString(String);

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

impl Valuelike for ShortString {
    fn from_value(value: Value) -> Self {
        ShortString(
            String::from_utf8(
                IntoIterator::into_iter(value)
                    .take_while(|&x| x != 0)
                    .collect(),
            )
            .unwrap(),
        )
    }

    fn into_value(&self) -> Value {
        let mut data = [0; 32];
        let bytes = self.0.as_bytes();
        data[..bytes.len()].copy_from_slice(bytes);
        data
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

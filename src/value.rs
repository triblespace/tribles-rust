use std::fmt::Debug;


pub const VALUE_LEN: usize = 32;
pub type Value = [u8; VALUE_LEN];

/// A type that is convertible to and from a [Value].
pub trait Valuelike: Sized {
    fn from_value(value: Value) -> Result<Self, ValueParseError>;
    fn into_value(v: &Self) -> Value;
}

impl Valuelike for Value {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(value)
    }

    fn into_value(value: &Self) -> Value {
        *value
    }
}

pub struct ValueParseError {
    value: Value,
    msg: String,
}

impl ValueParseError {
    pub fn new(value: Value, msg: &str) -> Self {
        ValueParseError {
            value,
            msg: msg.to_owned(),
        }
    }
}

impl Eq for ValueParseError {}
impl PartialEq for ValueParseError {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.msg == other.msg
    }
}
impl Debug for ValueParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueParseError")
            .field("value", &hex::encode(&self.value))
            .field("msg", &self.msg)
            .finish()
    }
}
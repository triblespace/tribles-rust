use crate::types::{Value, Valuelike, ValueParseError};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct RawValue(pub Value);

impl Valuelike for RawValue {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(RawValue(value))
    }

    fn into_value(&self) -> Value {
        self.0
    }
}

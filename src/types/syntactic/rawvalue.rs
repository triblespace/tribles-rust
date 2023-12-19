use crate::types::{Value, Valuelike};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct RawValue(pub Value);

impl Valuelike for RawValue {
    fn from_value(value: Value) -> Self {
        RawValue(value)
    }

    fn into_value(&self) -> Value {
        self.0
    }
}

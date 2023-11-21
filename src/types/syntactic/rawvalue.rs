use crate::{trible::*, inline_value};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct RawValue(pub Value);

inline_value!(RawValue);

impl From<&RawValue> for Value {
    fn from(raw: &RawValue) -> Self {
        raw.0
    }
}

impl From<Value> for RawValue {
    fn from(value: Value) -> Self {
        RawValue(value)
    }
}

use crate::Value;
use f256::f256;

pub struct F256LE;
pub struct F256BE;

pub type F256 = F256BE;

impl From<Value<F256BE>> for f256 {
    fn from(value: Value<F256BE>) -> Self {
        f256::from_be_bytes(value.bytes)
    }
}

impl From<f256> for Value<F256BE> {
    fn from(value: f256) -> Self {
        Value::new(value.to_be_bytes())
    }
}

impl From<Value<F256LE>> for f256 {
    fn from(value: Value<F256LE>) -> Self {
        f256::from_le_bytes(value.bytes)
    }
}

impl From<f256> for Value<F256LE> {
    fn from(value: f256) -> Self {
        Value::new(value.to_le_bytes())
    }
}

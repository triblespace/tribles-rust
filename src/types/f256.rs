use crate::Valuelike;
use f256::f256;

impl Valuelike for f256 {
    fn from_value(bytes: crate::Value) -> Result<Self, crate::ValueParseError> {
        Ok(f256::from_be_bytes(bytes))
    }

    fn into_value(n: &Self) -> crate::Value {
        n.to_be_bytes()
    }
}

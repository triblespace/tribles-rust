use crate::{
    id::Id,
    id_hex,
    value::{FromValue, ToValue, Value, ValueSchema},
};

use f256::f256;

/// A value schema for a 256-bit floating point number in little-endian byte order.
pub struct F256LE;

/// A value schema for a 256-bit floating point number in big-endian byte order.
pub struct F256BE;

/// A type alias for the little-endian version of the 256-bit floating point number.
pub type F256 = F256LE;

impl ValueSchema for F256LE {
    const VALUE_SCHEMA_ID: Id = id_hex!("D9A419D3CAA0D8E05D8DAB950F5E80F2");
}
impl ValueSchema for F256BE {
    const VALUE_SCHEMA_ID: Id = id_hex!("A629176D4656928D96B155038F9F2220");
}

impl FromValue<'_, F256BE> for f256 {
    fn from_value(v: &Value<F256BE>) -> Self {
        f256::from_be_bytes(v.raw)
    }
}

impl ToValue<F256BE> for f256 {
    fn to_value(self) -> Value<F256BE> {
        Value::new(self.to_be_bytes())
    }
}

impl FromValue<'_, F256LE> for f256 {
    fn from_value(v: &Value<F256LE>) -> Self {
        f256::from_le_bytes(v.raw)
    }
}

impl ToValue<F256LE> for f256 {
    fn to_value(self) -> Value<F256LE> {
        Value::new(self.to_le_bytes())
    }
}

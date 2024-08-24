use crate::Value;
use f256::f256;

use crate::Schema;

use super::{Pack, Unpack};

pub struct F256LE;
pub struct F256BE;

pub type F256 = F256BE;

impl Schema for F256LE {}
impl Schema for F256BE {}

impl Unpack<'_, F256BE> for f256 {
    fn unpack(v: &Value<F256BE>) -> Self {
        f256::from_be_bytes(v.bytes)
    }
}

impl Pack<F256BE> for f256 {
    fn pack(&self) -> Value<F256BE> {
        Value::new(self.to_be_bytes())
    }
}

impl Unpack<'_, F256LE> for f256 {
    fn unpack(v: &Value<F256LE>) -> Self {
        f256::from_le_bytes(v.bytes)
    }
}

impl Pack<F256LE> for f256 {
    fn pack(&self) -> Value<F256LE> {
        Value::new(self.to_le_bytes())
    }
}

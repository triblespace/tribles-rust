use std::convert::TryInto;

use crate::{ValueSchema, Value};
use num_rational::Ratio;

use super::{Pack, Unpack};

pub struct FR256LE;
pub struct FR256BE;

pub type FR256 = FR256LE;

impl ValueSchema for FR256LE {}
impl ValueSchema for FR256BE {}

impl Unpack<'_, FR256BE> for Ratio<i128> {
    fn unpack(v: &Value<FR256BE>) -> Self {
        let n = i128::from_be_bytes(v.bytes[0..16].try_into().unwrap());
        let d = i128::from_be_bytes(v.bytes[16..32].try_into().unwrap());

        Ratio::new(n, d)
    }
}

impl Pack<FR256BE> for Ratio<i128> {
    fn pack(&self) -> Value<FR256BE> {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.numer().to_be_bytes());
        bytes[16..32].copy_from_slice(&self.denom().to_be_bytes());

        Value::new(bytes)
    }
}

impl Unpack<'_, FR256LE> for Ratio<i128> {
    fn unpack(v: &Value<FR256LE>) -> Self {
        let n = i128::from_le_bytes(v.bytes[0..16].try_into().unwrap());
        let d = i128::from_le_bytes(v.bytes[16..32].try_into().unwrap());

        Ratio::new(n, d)
    }
}

impl Pack<FR256LE> for Ratio<i128> {
    fn pack(&self) -> Value<FR256LE> {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.numer().to_le_bytes());
        bytes[16..32].copy_from_slice(&self.denom().to_le_bytes());

        Value::new(bytes)
    }
}

use std::convert::TryInto;

use crate::Value;
use num_rational::Ratio;

pub struct FR256LE;
pub struct FR256BE;

pub type FR256 = FR256LE;

impl From<Value<FR256BE>> for Ratio<i128> {
    fn from(value: Value<FR256BE>) -> Self {
        let n = i128::from_be_bytes(value.bytes[0..16].try_into().unwrap());
        let d = i128::from_be_bytes(value.bytes[16..32].try_into().unwrap());

        Ratio::new(n, d)
    }
}

impl From<Ratio<i128>> for Value<FR256BE> {
    fn from(value: Ratio<i128>) -> Self {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&value.numer().to_be_bytes());
        bytes[16..32].copy_from_slice(&value.denom().to_be_bytes());

        Value::new(bytes)
    }
}

impl From<Value<FR256LE>> for Ratio<i128> {
    fn from(value: Value<FR256LE>) -> Self {
        let n = i128::from_le_bytes(value.bytes[0..16].try_into().unwrap());
        let d = i128::from_le_bytes(value.bytes[16..32].try_into().unwrap());

        Ratio::new(n, d)
    }
}

impl From<Ratio<i128>> for Value<FR256LE> {
    fn from(value: Ratio<i128>) -> Self {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&value.numer().to_le_bytes());
        bytes[16..32].copy_from_slice(&value.denom().to_le_bytes());

        Value::new(bytes)
    }
}

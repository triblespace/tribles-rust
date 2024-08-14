use crate::Value;
use ethnum;

pub struct U256LE;
pub struct U256BE;
pub struct I256LE;
pub struct I256BE;

pub type I256 = I256BE;
pub type U256 = U256BE;


impl From<ethnum::U256> for Value<U256BE> {
    fn from(value: ethnum::U256) -> Self {
        Value::new(value.to_be_bytes())
    }
}

impl From<Value<U256BE>> for ethnum::U256 {
    fn from(value: Value<U256BE>) -> Self {
        ethnum::U256::from_be_bytes(value.bytes)
    }
}

impl From<ethnum::U256> for Value<U256LE> {
    fn from(value: ethnum::U256) -> Self {
        Value::new(value.to_le_bytes())
    }
}

impl From<Value<U256LE>> for ethnum::U256 {
    fn from(value: Value<U256LE>) -> Self {
        ethnum::U256::from_le_bytes(value.bytes)
    }
}

impl From<ethnum::I256> for Value<I256BE> {
    fn from(value: ethnum::I256) -> Self {
        Value::new(value.to_be_bytes())
    }
}

impl From<Value<I256BE>> for ethnum::I256 {
    fn from(value: Value<I256BE>) -> Self {
        ethnum::I256::from_be_bytes(value.bytes)
    }
}

impl From<ethnum::I256> for Value<I256LE> {
    fn from(value: ethnum::I256) -> Self {
        Value::new(value.to_le_bytes())
    }
}

impl From<Value<I256LE>> for ethnum::I256 {
    fn from(value: Value<I256LE>) -> Self {
        ethnum::I256::from_le_bytes(value.bytes)
    }
}

impl From<u8> for Value<U256LE> {
    fn from(value: u8) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_le_bytes())
    }
}

impl From<u16> for Value<U256LE> {
    fn from(value: u16) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_le_bytes())
    }
}

impl From<u32> for Value<U256LE> {
    fn from(value: u32) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_le_bytes())
    }
}

impl From<u64> for Value<U256LE> {
    fn from(value: u64) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_le_bytes())
    }
}

impl From<u128> for Value<U256LE> {
    fn from(value: u128) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_le_bytes())
    }
}

impl From<u8> for Value<U256BE> {
    fn from(value: u8) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_be_bytes())
    }
}

impl From<u16> for Value<U256BE> {
    fn from(value: u16) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_be_bytes())
    }
}

impl From<u32> for Value<U256BE> {
    fn from(value: u32) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_be_bytes())
    }
}

impl From<u64> for Value<U256BE> {
    fn from(value: u64) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_be_bytes())
    }
}

impl From<u128> for Value<U256BE> {
    fn from(value: u128) -> Self {
        Value::new(ethnum::U256::new(value.into()).to_be_bytes())
    }
}

impl From<i8> for Value<I256LE> {
    fn from(value: i8) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_le_bytes())
    }
}

impl From<i16> for Value<I256LE> {
    fn from(value: i16) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_le_bytes())
    }
}

impl From<i32> for Value<I256LE> {
    fn from(value: i32) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_le_bytes())
    }
}

impl From<i64> for Value<I256LE> {
    fn from(value: i64) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_le_bytes())
    }
}

impl From<i128> for Value<I256LE> {
    fn from(value: i128) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_le_bytes())
    }
}

impl From<i8> for Value<I256BE> {
    fn from(value: i8) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_be_bytes())
    }
}

impl From<i16> for Value<I256BE> {
    fn from(value: i16) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_be_bytes())
    }
}

impl From<i32> for Value<I256BE> {
    fn from(value: i32) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_be_bytes())
    }
}

impl From<i64> for Value<I256BE> {
    fn from(value: i64) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_be_bytes())
    }
}

impl From<i128> for Value<I256BE> {
    fn from(value: i128) -> Self {
        Value::new(ethnum::I256::new(value.into()).to_be_bytes())
    }
}
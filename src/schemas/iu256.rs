use crate::{ Value, Schema };
use ethnum;

use super::{Pack, Unpack};

pub struct U256LE;
pub struct U256BE;
pub struct I256LE;
pub struct I256BE;

pub type I256 = I256BE;
pub type U256 = U256BE;

impl Schema for U256LE {}
impl Schema for U256BE {}
impl Schema for I256LE {}
impl Schema for I256BE {}

impl Pack<U256BE> for ethnum::U256 {    
    fn pack(&self) -> Value<U256BE> {
        Value::new(self.to_be_bytes())
    }
}

impl Unpack<'_, U256BE> for ethnum::U256 {
    fn unpack(v: &Value<U256BE>) -> Self {
        ethnum::U256::from_be_bytes(v.bytes)
    }
}

impl Pack<U256LE> for ethnum::U256 {
    fn pack(&self) -> Value<U256LE> {
        Value::new(self.to_le_bytes())
    }
}

impl Unpack<'_, U256LE> for ethnum::U256 {    
    fn unpack(v: &Value<U256LE>) -> Self {
        ethnum::U256::from_le_bytes(v.bytes)
    }
}

impl Pack<I256BE> for ethnum::I256{
    fn pack(&self) -> Value<I256BE> {
        Value::new(self.to_be_bytes())
    }
}

impl Unpack<'_, I256BE> for ethnum::I256 {
    fn unpack(v: &Value<I256BE>) -> Self {
        ethnum::I256::from_be_bytes(v.bytes)
    }
}

impl Pack<I256LE> for ethnum::I256 {
    fn pack(&self) -> Value<I256LE> {
        Value::new(self.to_le_bytes())
    }
}

impl Unpack<'_, I256LE> for ethnum::I256 {
    fn unpack(v: &Value<I256LE>) -> Self {
        ethnum::I256::from_le_bytes(v.bytes)
    }
}

impl Pack<U256LE> for u8 {    
    fn pack(&self) -> Value<U256LE> {
        Value::new(ethnum::U256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<U256LE> for u16 {    
    fn pack(&self) -> Value<U256LE> {
        Value::new(ethnum::U256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<U256LE> for u32 {    
    fn pack(&self) -> Value<U256LE> {
        Value::new(ethnum::U256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<U256LE> for u64 {    
    fn pack(&self) -> Value<U256LE> {
        Value::new(ethnum::U256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<U256LE> for u128 {    
    fn pack(&self) -> Value<U256LE> {
        Value::new(ethnum::U256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<U256BE> for u8 {    
    fn pack(&self) -> Value<U256BE> {
        Value::new(ethnum::U256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<U256BE> for u16 {    
    fn pack(&self) -> Value<U256BE> {
        Value::new(ethnum::U256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<U256BE> for u32 {    
    fn pack(&self) -> Value<U256BE> {
        Value::new(ethnum::U256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<U256BE> for u64 {    
    fn pack(&self) -> Value<U256BE> {
        Value::new(ethnum::U256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<U256BE> for u128 {    
    fn pack(&self) -> Value<U256BE> {
        Value::new(ethnum::U256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<I256LE> for i8 {    
    fn pack(&self) -> Value<I256LE> {
        Value::new(ethnum::I256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<I256LE> for i16 {    
    fn pack(&self) -> Value<I256LE> {
        Value::new(ethnum::I256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<I256LE> for i32 {    
    fn pack(&self) -> Value<I256LE> {
        Value::new(ethnum::I256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<I256LE> for i64 {    
    fn pack(&self) -> Value<I256LE> {
        Value::new(ethnum::I256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<I256LE> for i128 {    
    fn pack(&self) -> Value<I256LE> {
        Value::new(ethnum::I256::new((*self).into()).to_le_bytes())
    }
}

impl Pack<I256BE> for i8 {    
    fn pack(&self) -> Value<I256BE> {
        Value::new(ethnum::I256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<I256BE> for i32 {    
    fn pack(&self) -> Value<I256BE> {
        Value::new(ethnum::I256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<I256BE> for i64 {    
    fn pack(&self) -> Value<I256BE> {
        Value::new(ethnum::I256::new((*self).into()).to_be_bytes())
    }
}

impl Pack<I256BE> for i128 {    
    fn pack(&self) -> Value<I256BE> {
        Value::new(ethnum::I256::new((*self).into()).to_be_bytes())
    }
}

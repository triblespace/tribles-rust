use crate::id::RawId;
use crate::value::{ Value, ValueSchema, PackValue, UnpackValue };

use f256::f256;
use hex_literal::hex;

pub struct F256LE;
pub struct F256BE;

pub type F256 = F256BE;

impl ValueSchema for F256LE {const ID: RawId = hex!("D9A419D3CAA0D8E05D8DAB950F5E80F2");}
impl ValueSchema for F256BE {const ID: RawId = hex!("A629176D4656928D96B155038F9F2220");}

impl UnpackValue<'_, F256BE> for f256 {
    fn unpack(v: &Value<F256BE>) -> Self {
        f256::from_be_bytes(v.bytes)
    }
}

impl PackValue<F256BE> for f256 {
    fn pack(&self) -> Value<F256BE> {
        Value::new(self.to_be_bytes())
    }
}

impl UnpackValue<'_, F256LE> for f256 {
    fn unpack(v: &Value<F256LE>) -> Self {
        f256::from_le_bytes(v.bytes)
    }
}

impl PackValue<F256LE> for f256 {
    fn pack(&self) -> Value<F256LE> {
        Value::new(self.to_le_bytes())
    }
}

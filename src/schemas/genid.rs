use std::convert::TryFrom;
use std::convert::TryInto;

pub use crate::genid::genid;
use hex::FromHexError;
use hex::FromHex;
pub use crate::ufoid::ufoid;
pub use crate::fucid;
use crate::RawId;

use rand::RngCore;

use crate::schemas::TryPack;
use crate::schemas::TryUnpack;
use crate::{Schema, Value, VALUE_LEN};

use super::Pack;

pub struct GenId;

impl Schema for GenId {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GenIdParseError {
    IsNil,
    BadFormat
}

impl TryFrom<&Value<GenId>> for RawId {    
    type Error = GenIdParseError;
    
    fn try_from(value: &Value<GenId>) -> Result<Self, Self::Error> {
        if value.bytes[0..16] != [0; 16] {
            return Err(GenIdParseError::BadFormat)
        }
        if value.bytes[16..32] == [0; 16] {
            return Err(GenIdParseError::IsNil)
        }
        Ok(value.bytes[16..32].try_into().unwrap())
    }
}

impl From<&RawId> for Value<GenId> {
    fn from(value: &RawId) -> Self {
        let mut data = [0; VALUE_LEN];
        data[16..32].copy_from_slice(&value[..]);
        Value::new(data)
    }
}

impl From<RawId> for Value<GenId> {
    fn from(value: RawId) -> Self {
        (&value).into()
    }
}

impl TryUnpack<'_, GenId> for RawId {
    type Error = GenIdParseError;
    
    fn try_unpack(v: &'_ Value<GenId>) -> Result<Self, Self::Error> {
        v.try_into()
    }
}

impl Pack<GenId> for RawId {    
    fn pack(&self) -> Value<GenId> {
        self.into()
    }
}

impl TryUnpack<'_, GenId> for String {
    type Error = GenIdParseError;
    
    fn try_unpack(v: &'_ Value<GenId>) -> Result<Self, Self::Error> {
        let id: RawId = v.try_into()?;
        let mut s = String::new();
        s.push_str("genid:");
        s.push_str(&hex::encode(id));
        Ok(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackIdError {
    BadProtocol,
    BadHex(FromHexError)
}

impl From<FromHexError> for PackIdError {
    fn from(value: FromHexError) -> Self {
        PackIdError::BadHex(value)
    }
}

impl TryPack<GenId> for str {
    type Error = PackIdError;
    
    fn try_pack(&self) -> Result<Value<GenId>, Self::Error> {
        let protocol = "genid:";
        if !self.starts_with(protocol){
            return Err(PackIdError::BadProtocol)
        }
       let id = RawId::from_hex(&self[protocol.len()..])?;
       Ok(id.into())
    }
}

#[cfg(feature = "proptest")]
pub struct IdValueTree(RawId);

#[cfg(feature = "proptest")]
#[derive(Debug)]
pub struct RandomGenId();
#[cfg(feature = "proptest")]
impl proptest::strategy::Strategy for RandomGenId {
    type Tree = IdValueTree;
    type Value = RawId;

    fn new_tree(
        &self,
        runner: &mut proptest::prelude::prop::test_runner::TestRunner,
    ) -> proptest::prelude::prop::strategy::NewTree<Self> {
        let rng = runner.rng();
        let mut id = [0; 16];
        rng.fill_bytes(&mut id[..]);

        Ok(IdValueTree(id.into()))
    }
}

#[cfg(feature = "proptest")]
impl proptest::strategy::ValueTree for IdValueTree {
    type Value = RawId;

    fn simplify(&mut self) -> bool {
        false
    }
    fn complicate(&mut self) -> bool {
        false
    }
    fn current(&self) -> RawId {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(genid() != genid());
    }
}

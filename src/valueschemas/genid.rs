use std::convert::TryFrom;
use std::convert::TryInto;

pub use crate::fucid;
pub use crate::genid::genid;
pub use crate::ufoid::ufoid;
use crate::RawId;
use hex::FromHex;
use hex::FromHexError;

use rand::RngCore;

use crate::valueschemas::TryPackValue;
use crate::valueschemas::TryUnpackValue;
use crate::{Value, ValueSchema, VALUE_LEN};

use super::PackValue;

pub struct GenId;

impl ValueSchema for GenId {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GenIdParseError {
    IsNil,
    BadFormat,
}

impl TryFrom<&Value<GenId>> for RawId {
    type Error = GenIdParseError;

    fn try_from(value: &Value<GenId>) -> Result<Self, Self::Error> {
        if value.bytes[0..16] != [0; 16] {
            return Err(GenIdParseError::BadFormat);
        }
        if value.bytes[16..32] == [0; 16] {
            return Err(GenIdParseError::IsNil);
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

impl TryUnpackValue<'_, GenId> for RawId {
    type Error = GenIdParseError;

    fn try_unpack(v: &'_ Value<GenId>) -> Result<Self, Self::Error> {
        v.try_into()
    }
}

impl PackValue<GenId> for RawId {
    fn pack(&self) -> Value<GenId> {
        self.into()
    }
}

impl TryUnpackValue<'_, GenId> for String {
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
    BadHex(FromHexError),
}

impl From<FromHexError> for PackIdError {
    fn from(value: FromHexError) -> Self {
        PackIdError::BadHex(value)
    }
}

impl TryPackValue<GenId> for str {
    type Error = PackIdError;

    fn try_pack(&self) -> Result<Value<GenId>, Self::Error> {
        let protocol = "genid:";
        if !self.starts_with(protocol) {
            return Err(PackIdError::BadProtocol);
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

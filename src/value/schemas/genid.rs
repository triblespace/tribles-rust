use crate::id::{OwnedId, RawId};
use crate::value::{FromValue, ToValue, TryFromValue, TryToValue, Value, ValueSchema, VALUE_LEN};

use std::convert::TryInto;

use hex::FromHex;
use hex::FromHexError;

use rand::RngCore;

use hex_literal::hex;

pub struct GenId;

impl ValueSchema for GenId {
    const ID: RawId = hex!("B08EE1D45EB081E8C47618178AFE0D81");
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IdParseError {
    IsNil,
    BadFormat,
}

impl<'a> TryFromValue<'a, GenId> for &'a RawId {
    type Error = IdParseError;

    fn try_from_value(value: &'a Value<GenId>) -> Result<Self, Self::Error> {
        if value.bytes[0..16] != [0; 16] {
            return Err(IdParseError::BadFormat);
        }
        if value.bytes[16..32] == [0; 16] {
            return Err(IdParseError::IsNil);
        }
        Ok(value.bytes[16..32].try_into().unwrap())
    }
}

impl TryFromValue<'_, GenId> for RawId {
    type Error = IdParseError;

    fn try_from_value(value: &Value<GenId>) -> Result<Self, Self::Error> {
        let r: Result<&RawId, IdParseError> = value.try_from_value();
        r.copied()
    }
}

impl FromValue<'_, GenId> for RawId {
    fn from_value(v: &Value<GenId>) -> Self {
        v.try_from_value().unwrap()
    }
}

impl<'a> FromValue<'a, GenId> for &'a RawId {
    fn from_value(v: &'a Value<GenId>) -> Self {
        v.try_from_value().unwrap()
    }
}

impl ToValue<GenId> for RawId {
    fn to_value(self) -> Value<GenId> {
        let mut data = [0; VALUE_LEN];
        data[16..32].copy_from_slice(&self[..]);
        Value::new(data)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OwnedIdError {
    FailedParse(IdParseError),
    FailedAquire(),
}

impl From<IdParseError> for OwnedIdError {
    fn from(e: IdParseError) -> Self {
        OwnedIdError::FailedParse(e)
    }
}

impl<'a> TryFromValue<'a, GenId> for OwnedId {
    type Error = OwnedIdError;

    fn try_from_value(value: &'a Value<GenId>) -> Result<Self, Self::Error> {
        let id: RawId = value.try_from_value()?;
        OwnedId::try_aquire(id).ok_or(OwnedIdError::FailedAquire())
    }
}

impl FromValue<'_, GenId> for OwnedId {
    fn from_value(v: &Value<GenId>) -> Self {
        v.try_from_value().unwrap()
    }
}

impl ToValue<GenId> for OwnedId {
    fn to_value(self) -> Value<GenId> {
        let id: RawId = *self;
        id.to_value()
    }
}

impl ToValue<GenId> for &OwnedId {
    fn to_value(self) -> Value<GenId> {
        let id: RawId = **self;
        id.to_value()
    }
}

impl TryFromValue<'_, GenId> for String {
    type Error = IdParseError;

    fn try_from_value(v: &'_ Value<GenId>) -> Result<Self, Self::Error> {
        let id: RawId = v.try_from_value()?;
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

impl TryToValue<GenId> for &str {
    type Error = PackIdError;

    fn try_to_value(self) -> Result<Value<GenId>, Self::Error> {
        let protocol = "genid:";
        if !self.starts_with(protocol) {
            return Err(PackIdError::BadProtocol);
        }
        let id = RawId::from_hex(&self[protocol.len()..])?;
        Ok(id.to_value())
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
    use crate::id::rngid;

    #[test]
    fn unique() {
        assert!(rngid() != rngid());
    }
}

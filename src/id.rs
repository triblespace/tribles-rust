pub mod fucid;
pub mod ufoid;

use std::convert::TryFrom;
use std::convert::TryInto;

pub use fucid::fucid;
pub use ufoid::ufoid;

use rand::thread_rng;
use rand::RngCore;

use crate::RawValue;
use crate::Value;
use crate::VALUE_LEN;

pub const ID_LEN: usize = 16;
pub type RawId = [u8; ID_LEN];

pub fn id_into_value(id: RawId) -> RawValue {
    let mut data = [0; VALUE_LEN];
    data[16..32].copy_from_slice(&id[..]);
    data
}

pub fn id_from_value(id: RawValue) -> Option<RawId> {
    if id[0..16] != [0; 16] {
        return None;
    }
    let id = id[16..32].try_into().unwrap();
    Some(id)
}

pub struct Id;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RndIdParseError {
    IsNil,
    BadFormat
}

impl TryFrom<Value<Id>> for RawId {    
    type Error = RndIdParseError;
    
    fn try_from(value: Value<Id>) -> Result<Self, Self::Error> {
        if value.bytes[0..16] != [0; 16] {
            return Err(RndIdParseError::BadFormat)
        }
        if value.bytes[16..32] == [0; 16] {
            return Err(RndIdParseError::IsNil)
        }
        Ok(value.bytes[16..32].try_into().unwrap())
    }
}

impl From<RawId> for Value<Id> {
    fn from(value: RawId) -> Self {
        let mut data = [0; VALUE_LEN];
        data[16..32].copy_from_slice(&value[..]);
        Value::new(data)
    }
}

pub fn idgen() -> RawId {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    id
}

#[cfg(feature = "proptest")]
pub struct IdValueTree(RawId);

#[cfg(feature = "proptest")]
#[derive(Debug)]
pub struct RandomId();
#[cfg(feature = "proptest")]
impl proptest::strategy::Strategy for RandomId {
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
        assert!(idgen() != idgen());
    }
}

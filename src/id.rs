pub mod fucid;
pub mod ufoid;

use std::convert::TryInto;

pub use fucid::fucid;
pub use ufoid::ufoid;

use rand::thread_rng;
use rand::RngCore;

use crate::Value;
use crate::ValueParseError;
use crate::Valuelike;
use crate::VALUE_LEN;

pub const ID_LEN: usize = 16;
pub type Id = [u8; ID_LEN];

pub fn id_into_value(id: Id) -> Value {
    let mut data = [0; VALUE_LEN];
    data[16..32].copy_from_slice(&id[..]);
    data
}

pub fn id_from_value(id: Value) -> Id {
    id[16..32].try_into().unwrap()
}

impl Valuelike for Id {
    fn from_value(value: Value) -> Result<Self, ValueParseError> {
        Ok(value[16..32].try_into().unwrap())
    }

    fn into_value(id: &Self) -> Value {
        id_into_value(*id)
    }
}

pub fn idgen() -> Id {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    id
}

#[cfg(feature = "proptest")]
pub struct IdValueTree(Id);

#[cfg(feature = "proptest")]
#[derive(Debug)]
pub struct RandId();
#[cfg(feature = "proptest")]
impl proptest::strategy::Strategy for RandId {
    type Tree = IdValueTree;
    type Value = Id;

    fn new_tree(
        &self,
        runner: &mut proptest::prelude::prop::test_runner::TestRunner,
    ) -> proptest::prelude::prop::strategy::NewTree<Self> {
        let rng = runner.rng();
        let mut id = [0; 16];
        rng.fill_bytes(&mut id[..]);

        Ok(IdValueTree(id))
    }
}

#[cfg(feature = "proptest")]
impl proptest::strategy::ValueTree for IdValueTree {
    type Value = Id;

    fn simplify(&mut self) -> bool {
        false
    }
    fn complicate(&mut self) -> bool {
        false
    }
    fn current(&self) -> Id {
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

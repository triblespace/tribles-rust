pub mod fucid;
pub mod rngid;
pub mod ufoid;

use std::convert::TryInto;

pub use fucid::fucid;
pub use rngid::rngid;
pub use ufoid::ufoid;

use crate::value::{RawValue, VALUE_LEN};

pub const ID_LEN: usize = 16;
pub type RawId = [u8; ID_LEN];

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FreshId {
    pub raw: RawId,
    _private: ()
}

impl FreshId {
    pub unsafe fn new(id: RawId) -> Self {
        FreshId {raw: id, _private: ()}
    }
}

unsafe impl Send for FreshId {}

impl From<FreshId> for RawId {
    fn from(value: FreshId) -> Self {
        value.raw
    }
}

pub(crate) fn id_into_value(id: &RawId) -> RawValue {
    let mut data = [0; VALUE_LEN];
    data[16..32].copy_from_slice(id);
    data
}

pub(crate) fn id_from_value(id: &RawValue) -> Option<RawId> {
    if id[0..16] != [0; 16] {
        return None;
    }
    let id = id[16..32].try_into().unwrap();
    Some(id)
}

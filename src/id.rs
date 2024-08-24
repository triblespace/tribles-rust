pub mod genid;
pub mod fucid;
pub mod ufoid;

use std::convert::TryInto;

pub use genid::genid;
pub use ufoid::ufoid;
pub use fucid::fucid;

use crate::{RawValue, VALUE_LEN};

pub const ID_LEN: usize = 16;
pub type RawId = [u8; ID_LEN];

pub(crate) fn id_into_value(id: RawId) -> RawValue {
    let mut data = [0; VALUE_LEN];
    data[16..32].copy_from_slice(&id[..]);
    data
}

pub(crate) fn id_from_value(id: RawValue) -> Option<RawId> {
    if id[0..16] != [0; 16] {
        return None;
    }
    let id = id[16..32].try_into().unwrap();
    Some(id)
}

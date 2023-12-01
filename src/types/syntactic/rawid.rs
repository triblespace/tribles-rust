use crate::inline_value;
use crate::namespace::*;
use crate::trible::*;
use arbitrary::Arbitrary;
use rand::RngCore;
use rand::thread_rng;
use std::convert::TryInto;

// Universal Forgettable Ordered IDs
#[derive(Arbitrary, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct RawId(pub Id);

inline_value!(RawId);

impl RawId {
    pub const fn raw(data: [u8; 16]) -> RawId {
        RawId(data)
    }

    pub fn new() -> RawId {
        let mut rng = thread_rng();
        let mut id = RawId([0; 16]);
        rng.fill_bytes(&mut id.0[..]);

        return id;
    }
}

impl From<Id> for RawId {
    fn from(data: Id) -> Self {
        RawId(data)
    }
}

impl From<&RawId> for Id {
    fn from(id: &RawId) -> Self {
        id.0
    }
}

impl Factory for RawId {
    fn factory() -> Self {
        RawId::new()
    }
}

impl From<Value> for RawId {
    fn from(data: Value) -> Self {
        RawId(data[16..32].try_into().unwrap())
    }
}

impl From<&RawId> for Value {
    fn from(id: &RawId) -> Self {
        let mut data = [0; 32];
        data[16..32].copy_from_slice(&id.0);
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(RawId::new() != RawId::new());
    }
}

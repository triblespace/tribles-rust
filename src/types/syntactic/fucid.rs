use crate::namespace::*;
use arbitrary::Arbitrary;
use rand::thread_rng;
use rand::RngCore;
use std::cell::RefCell;
use std::convert::TryInto;

struct FUCIDgen {
    counter: u128,
    salt: u128,
}

thread_local!(static GEN_STATE: RefCell<FUCIDgen> = RefCell::new(FUCIDgen {
    counter: 0,
    salt: {
        let mut rng = thread_rng();
        let mut rand_bytes = [0; 16];
        rng.fill_bytes(&mut rand_bytes[..]);

        u128::from_be_bytes(rand_bytes)
    }
}));

// Fast Unsafe Compressable IDs
#[derive(Arbitrary, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct FUCID {
    data: [u8; 16],
}

impl FUCID {
    pub fn new() -> FUCID {
        FUCID {
            data: GEN_STATE
                .with(|cell| {
                    let mut state = cell.borrow_mut();
                    let next_id = state.counter ^ state.salt;
                    state.counter += 1;

                    next_id
                })
                .to_be_bytes(),
        }
    }
}

impl From<Id> for FUCID {
    fn from(data: Id) -> Self {
        FUCID { data }
    }
}

impl From<&FUCID> for Id {
    fn from(id: &FUCID) -> Self {
        id.data
    }
}

impl Factory for FUCID {
    fn factory() -> Self {
        FUCID::new()
    }
}

impl From<Value> for FUCID {
    fn from(data: Value) -> Self {
        FUCID {
            data: data[16..32].try_into().unwrap(),
        }
    }
}

impl From<&FUCID> for Value {
    fn from(id: &FUCID) -> Self {
        let mut data = [0; 32];
        data[16..32].copy_from_slice(&id.data);
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(FUCID::new() != FUCID::new());
    }
}

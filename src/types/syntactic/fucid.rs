use crate::types::{Id, Idlike};
use arbitrary::Arbitrary;
use rand::thread_rng;
use rand::RngCore;
use std::cell::RefCell;

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
#[derive(Arbitrary, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct FUCID([u8; 16]);

impl FUCID {
    pub const fn raw(data: [u8; 16]) -> FUCID {
        FUCID(data)
    }

    pub fn new() -> FUCID {
        FUCID(
            GEN_STATE
                .with(|cell| {
                    let mut state = cell.borrow_mut();
                    let next_id = state.counter ^ state.salt;
                    state.counter += 1;

                    next_id
                })
                .to_be_bytes(),
        )
    }
}

impl Idlike for FUCID {
    fn from_id(id: Id) -> Self {
        FUCID(id)
    }
    fn into_id(&self) -> Id {
        self.0
    }
    fn factory() -> Self {
        FUCID::new()
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

use rand::thread_rng;
use rand::RngCore;
use std::cell::RefCell;
use arbitrary::Arbitrary;
use std::convert::TryInto;
use crate::trible::{Id, Value};

struct FUCIDgen {
    counter: u128,
    salt: u128
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

#[derive(Arbitrary, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct FUCID {
    data: [u8; 16],
}

impl FUCID {
    pub fn new() -> FUCID {
        FUCID {
            data: GEN_STATE.with(|cell| {
                let mut state = cell.borrow_mut();
                let next_id = state.counter ^ state.salt;
                state.counter += 1;
    
                next_id
            }).to_be_bytes()
        }
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

impl Id for FUCID {
    fn decode(data: [u8; 16]) -> Self {
        FUCID {data}
    }
    fn encode(id: &Self) -> [u8; 16] {
        id.data
    }
}

impl Value for FUCID {
    fn decode(data: [u8; 32]) -> Self {
        FUCID {data: data[16..32].try_into().unwrap()}
    }
    fn encode(value: &Self) -> [u8; 32] {
        let mut data = [0; 32];
        data[16..32].copy_from_slice(&value.data);
        data
    }
}

/*
    pub fn decode(data: *const [32]u8) FUCID {
        return FUCID{.data = data.*};
    }

    pub fn encode(self: *const FUCID) [32]u8 {
        return self.data;
    }   
*/
use rand::thread_rng;
use rand::RngCore;
use std::cell::RefCell;

use super::OwnedId;

pub struct FUCIDgen {
    salt: u128,
    counter: u128,
}

impl FUCIDgen {
    pub fn new() -> Self {
        Self {
            salt: {
                let mut rng = thread_rng();
                let mut rand_bytes = [0; 16];
                rng.fill_bytes(&mut rand_bytes[..]);

                u128::from_be_bytes(rand_bytes)
            },
            counter: 0,
        }
    }

    pub fn new_salted(salt: [u8; 16]) -> Self {
        Self {
            salt: u128::from_be_bytes(salt),
            counter: 0,
        }
    }

    pub fn next(&mut self) -> OwnedId {
        let next_id = self.counter ^ self.salt;
        self.counter += 1;
        let id = next_id.to_be_bytes();
        OwnedId::force(id)
    }
}

thread_local!(static GEN_STATE: RefCell<FUCIDgen> = RefCell::new(FUCIDgen::new()));

/// Fast Unsafe Compressable IDs
pub fn fucid() -> OwnedId {
    GEN_STATE.with_borrow_mut(|gen| gen.next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(fucid() != fucid());
    }
}

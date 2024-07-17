use crate::RawId;

use rand::thread_rng;
use rand::RngCore;
use std::cell::RefCell;

pub struct FUCIDgen {
    counter: u128,
    salt: u128,
}

impl FUCIDgen {
    pub fn new() -> Self {
        Self {
            counter: 0,
            salt: {
                let mut rng = thread_rng();
                let mut rand_bytes = [0; 16];
                rng.fill_bytes(&mut rand_bytes[..]);
        
                u128::from_be_bytes(rand_bytes)
            }
        }
    }

    pub fn next(&mut self) -> RawId {
        let next_id = self.counter ^ self.salt;
        self.counter += 1;
        next_id.to_be_bytes()
    }
}

thread_local!(static GEN_STATE: RefCell<FUCIDgen> = RefCell::new(FUCIDgen::new()));

/// Fast Unsafe Compressable IDs
pub fn fucid() -> RawId {
    GEN_STATE
        .with(|cell| {
            let mut gen = cell.borrow_mut();
            gen.next()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(fucid() != fucid());
    }
}

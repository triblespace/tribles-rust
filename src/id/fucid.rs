use crate::Id;

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

/// Fast Unsafe Compressable IDs
pub fn fucid() -> Id {
    GEN_STATE
        .with(|cell| {
            let mut state = cell.borrow_mut();
            let next_id = state.counter ^ state.salt;
            state.counter += 1;

            next_id
        })
        .to_be_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(fucid() != fucid());
    }
}

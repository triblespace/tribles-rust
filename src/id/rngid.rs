use rand::{thread_rng, RngCore};

use super::{Id, OwnedId};

pub fn rngid() -> OwnedId {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    OwnedId::force(
        Id::new(id).expect("The probability for rng = 0 should be neglegible."),
    )
}

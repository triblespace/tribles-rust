use rand::{thread_rng, RngCore};

use super::FreshId;

pub fn rngid() -> FreshId {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    unsafe { FreshId::new(id) }
}

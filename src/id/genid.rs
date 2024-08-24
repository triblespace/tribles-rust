use crate::RawId;
use rand::{thread_rng, RngCore};

pub fn genid() -> RawId {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    id
}

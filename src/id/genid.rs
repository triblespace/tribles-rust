use rand::{thread_rng, RngCore};
use crate::RawId;

pub fn genid() -> RawId {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    id
}
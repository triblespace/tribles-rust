use crate::id::RawId;

use rand::{thread_rng, RngCore};
use std::time::{SystemTime, UNIX_EPOCH};

// Universal Forgettable Ordered IDs
pub fn ufoid() -> RawId {
    let mut rng = thread_rng();
    let now_in_sys = SystemTime::now();
    let now_since_epoch = now_in_sys
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");
    let now_in_ms = now_since_epoch.as_millis();

    let mut id = [0; 16];
    id[0..4].copy_from_slice(&(now_in_ms as u32).to_be_bytes());
    rng.fill_bytes(&mut id[4..16]);

    id.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(ufoid() != ufoid());
    }
}

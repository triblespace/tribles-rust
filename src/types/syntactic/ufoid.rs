use crate::types::Id;
use crate::types::Idlike;
use arbitrary::Arbitrary;
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};

// Universal Forgettable Ordered IDs
#[derive(Arbitrary, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct UFOID([u8; 16]);

impl UFOID {
    pub const fn raw(data: [u8; 16]) -> UFOID {
        UFOID(data)
    }

    pub fn new() -> UFOID {
        let mut rng = thread_rng();
        let now_in_sys = SystemTime::now();
        let now_since_epoch = now_in_sys
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");
        let now_in_ms = now_since_epoch.as_millis();

        return Self::new_with(now_in_ms as u32, &mut rng);
    }

    pub fn new_with(timestamp_ms: u32, rng: &mut dyn rand::RngCore) -> UFOID {
        let mut id = UFOID([0; 16]);
        id.0[0..4].copy_from_slice(&timestamp_ms.to_be_bytes());
        rng.fill_bytes(&mut id.0[4..16]);

        return id;
    }
}

impl Idlike for UFOID {
    fn from_id(id: Id) -> Self {
        UFOID(id)
    }

    fn into_id(&self) -> Id {
        self.0
    }

    fn factory() -> Self {
        UFOID::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(UFOID::new() != UFOID::new());
    }
}

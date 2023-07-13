use crate::namespace::*;
use arbitrary::Arbitrary;
use rand::thread_rng;
use std::convert::TryInto;
use std::time::{SystemTime, UNIX_EPOCH};

// Universal Forgettable Ordered IDs
#[derive(Arbitrary, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct UFOID {
    data: [u8; 16],
}

impl UFOID {
    pub const fn raw(data: [u8; 16]) -> UFOID {
        UFOID { data }
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
        let mut id = UFOID { data: [0; 16] };
        id.data[0..4].copy_from_slice(&timestamp_ms.to_be_bytes());
        rng.fill_bytes(&mut id.data[4..16]);

        return id;
    }
}

impl From<Id> for UFOID {
    fn from(data: Id) -> Self {
        UFOID { data }
    }
}

impl From<&UFOID> for Id {
    fn from(id: &UFOID) -> Self {
        id.data
    }
}

impl Factory for UFOID {
    fn factory() -> Self {
        UFOID::new()
    }
}

impl From<Value> for UFOID {
    fn from(data: Value) -> Self {
        UFOID {
            data: data[16..32].try_into().unwrap(),
        }
    }
}

impl From<&UFOID> for Value {
    fn from(id: &UFOID) -> Self {
        let mut data = [0; 32];
        data[16..32].copy_from_slice(&id.data);
        data
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

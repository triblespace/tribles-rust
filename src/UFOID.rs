use arbitrary::Arbitrary;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Arbitrary, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct UFOID {
    data: [u8; 16],
}

impl UFOID {
    pub fn new(rnd: &mut dyn rand::RngCore) -> UFOID {
        let now_in_sys = SystemTime::now();
        let now_since_epoch = now_in_sys
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");
        let now_in_ms = now_since_epoch.as_millis();

        return Self::newWithTime(now_in_ms as u32, rnd);
    }

    pub fn newWithTime(timestamp_ms: u32, rnd: &mut dyn rand::RngCore) -> UFOID {
        let mut id = UFOID { data: [0; 16] };
        id.data[0..4].copy_from_slice(&timestamp_ms.to_be_bytes());
        rnd.fill_bytes(&mut id.data[4..16]);

        return id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn unique() {
        let mut rng = thread_rng();
        assert!(UFOID::new(&mut rng) != UFOID::new(&mut rng));
    }
}

/*
    pub fn decode(data: *const [32]u8) UFOID {
        return UFOID{data = data[16..32]};
    }

    pub fn encode(self: *const UFOID) [32]u8 {
        return self.data;
    }
*/

use crate::types::Id;
use crate::types::Idlike;
use arbitrary::Arbitrary;
use rand::thread_rng;
use rand::RngCore;

// Universal Forgettable Ordered IDs
#[derive(Arbitrary, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct RawId(pub Id);

impl RawId {
    pub const fn raw(data: [u8; 16]) -> RawId {
        RawId(data)
    }

    pub fn new() -> RawId {
        let mut rng = thread_rng();
        let mut id = RawId([0; 16]);
        rng.fill_bytes(&mut id.0[..]);

        return id;
    }
}

impl Idlike for RawId {
    fn from_id(id: Id) -> Self {
        RawId(id)
    }

    fn into_id(&self) -> Id {
        self.0
    }

    fn factory() -> Self {
        RawId::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique() {
        assert!(RawId::new() != RawId::new());
    }
}

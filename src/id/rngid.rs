use rand::{thread_rng, RngCore};

use super::{ExclusiveId, Id};

/// # Random Number Generated ID (RNGID)
/// Are generated by simply taking 128bits from a cryptographic random
/// source. They are easy to implement and provide the maximum possible amount
/// of entropy at the cost of locality and compressability. However UFOIDs are
/// almost universally a better choice, unless the use-case is incompatible with
/// leaking the time at which an id was minted.
pub fn rngid() -> ExclusiveId {
    let mut rng = thread_rng();
    let mut id = [0; 16];
    rng.fill_bytes(&mut id[..]);

    ExclusiveId::force(Id::new(id).expect("The probability for rng = 0 should be neglegible."))
}

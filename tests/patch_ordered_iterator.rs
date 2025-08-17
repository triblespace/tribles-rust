use rand::{rngs::StdRng, RngCore, SeedableRng};
use std::collections::HashSet;
use tribles::patch::{Entry, PATCH};
use tribles::trible::EAVOrder;

#[test]
fn iter_ordered_returns_sorted_keys() {
    let mut patch: PATCH<64, EAVOrder> = PATCH::new();
    let mut rng = StdRng::seed_from_u64(0);
    let mut keys = HashSet::new();
    while keys.len() < 1000 {
        let mut key = [0u8; 64];
        rng.fill_bytes(&mut key);
        if keys.insert(key) {
            patch.insert(&Entry::new(&key));
        }
    }
    let mut sorted_keys: Vec<[u8; 64]> = keys.iter().cloned().collect();
    sorted_keys.sort();
    let collected: Vec<[u8; 64]> = patch.iter_ordered().cloned().collect();
    assert_eq!(collected, sorted_keys);
}

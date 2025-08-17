use rand::{rngs::StdRng, RngCore, SeedableRng};
use rand::rngs::ThreadRng;
use std::collections::HashSet;
use tribles::patch::IdentityOrder;
use tribles::trible::EAVOrder;
use tribles::patch::{Entry, PATCH};

#[test]
fn iter_ordered_returns_sorted_keys_eav() {
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

#[test]
fn iter_ordered_returns_sorted_keys_identity() {
    const N: usize = 128;
    let mut rng = ThreadRng::default();
    let mut patch: PATCH<64, IdentityOrder> = PATCH::new();
    let mut keys: Vec<[u8; 64]> = Vec::with_capacity(N);
    for _ in 0..N {
        let mut key = [0u8; 64];
        rng.fill_bytes(&mut key);
        patch.insert(&Entry::new(&key));
        keys.push(key);
    }
    keys.sort();
    let iter_keys: Vec<[u8; 64]> = patch.iter_ordered().map(|k| *k).collect();
    assert_eq!(keys, iter_keys);
}

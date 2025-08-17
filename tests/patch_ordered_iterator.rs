use rand::rngs::ThreadRng;
use rand::RngCore;
use tribles::patch::IdentityOrder;
use tribles::patch::{Entry, PATCH};

#[test]
fn iter_ordered_returns_sorted_keys() {
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

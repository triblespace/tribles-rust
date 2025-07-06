use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{commit, memoryrepo::MemoryRepo, Repository};

#[test]
fn branch_from_and_checkout_with_key() {
    // prepare storage with an initial commit
    let mut store = MemoryRepo::default();
    let key = SigningKey::generate(&mut OsRng);
    let commit_set = commit::commit(&key, [], None, None);
    let initial = store.put(commit_set).unwrap();

    let mut repo = Repository::new(store, key.clone());
    let mut ws = repo.branch_from("feature", initial).expect("branch from");
    ws.commit(TribleSet::new(), Some("work"));
    repo.push(&mut ws).expect("push");

    // checkout using a different key should succeed
    let other_key = SigningKey::generate(&mut OsRng);
    let branch_id = ws.branch_id();
    repo.checkout_with_key(branch_id, other_key)
        .expect("checkout");
}

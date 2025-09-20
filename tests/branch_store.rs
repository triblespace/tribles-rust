use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::memoryrepo::MemoryRepo;
use tribles::repo::PushResult;
use tribles::repo::{self};

#[test]
fn branch_update_success_and_conflict() {
    let mut store = MemoryRepo::default();
    let key = SigningKey::generate(&mut OsRng);
    let commit1 = repo::commit::commit_metadata(&key, [], None, None);
    let h1 = store.put(commit1).unwrap();
    let branch_id = *ufoid();

    match store.update(branch_id, None, h1) {
        Ok(PushResult::Success()) => {}
        _ => panic!("expected success"),
    }

    let commit2 = repo::commit::commit_metadata(&key, [h1], None, None);
    let h2 = store.put(commit2).unwrap();

    match store.update(branch_id, None, h2) {
        Ok(PushResult::Conflict(Some(existing))) => assert_eq!(existing, h1),
        r => panic!("unexpected result: {r:?}"),
    }

    match store.update(branch_id, Some(h1), h2) {
        Ok(PushResult::Success()) => {}
        _ => panic!("expected success"),
    }
}

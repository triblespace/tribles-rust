use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{self, RepoPushResult, Repository};

mod util;
use util::InMemoryRepo;

#[test]
fn push_and_merge_conflict_resolution() {
    let mut storage = InMemoryRepo::default();
    let signing_key = SigningKey::generate(&mut OsRng);
    let base_commit = repo::commit::commit(&signing_key, [], None, None);
    let base_handle = storage.put(base_commit).unwrap();
    let branch_key = SigningKey::generate(&mut OsRng);
    let mut repo = Repository::new(storage);
    let branch_id = repo.branch("main", base_handle, branch_key);

    let mut ws1 = match repo.checkout(branch_id) {
        Ok(ws) => ws,
        Err(_) => panic!("checkout failed"),
    };
    let mut ws2 = match repo.checkout(branch_id) {
        Ok(ws) => ws,
        Err(_) => panic!("checkout failed"),
    };

    ws1.commit(TribleSet::new(), Some("first"));
    ws2.commit(TribleSet::new(), Some("second"));

    match repo.push(&mut ws1).expect("push") {
        RepoPushResult::Success() => {}
        _ => panic!("expected success"),
    }

    let mut conflict_ws = match repo.push(&mut ws2).expect("push") {
        RepoPushResult::Conflict(ws) => ws,
        _ => panic!("expected conflict"),
    };

    conflict_ws.merge(&mut ws2).unwrap();

    match repo.push(&mut conflict_ws).expect("push") {
        RepoPushResult::Success() => {}
        _ => panic!("expected success after merge"),
    }
}

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::memoryrepo::MemoryRepo;
use tribles::repo::Repository;

#[test]
fn push_and_merge_conflict_resolution() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let mut ws1 = repo.branch("main").expect("create branch");
    let branch_id = ws1.branch_id();
    let mut ws2 = repo.pull(branch_id).expect("pull");

    ws1.commit(TribleSet::new(), Some("first"));
    ws2.commit(TribleSet::new(), Some("second"));

    match repo.push(&mut ws1).expect("push") {
        None => {}
        _ => panic!("expected success"),
    }

    let mut conflict_ws = match repo.push(&mut ws2).expect("push") {
        Some(ws) => ws,
        _ => panic!("expected conflict"),
    };

    conflict_ws.merge(&mut ws2).unwrap();

    match repo.push(&mut conflict_ws).expect("push") {
        None => {}
        _ => panic!("expected success after merge"),
    }
}

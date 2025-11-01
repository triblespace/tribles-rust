use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use triblespace::prelude::*;
use triblespace::repo::memoryrepo::MemoryRepo;
use triblespace::repo::Repository;

#[test]
fn push_and_merge_conflict_resolution() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws1 = repo.pull(*branch_id).expect("pull");
    let mut ws2 = repo.pull(*branch_id).expect("pull");

    ws1.commit(TribleSet::new(), Some("first"));
    ws2.commit(TribleSet::new(), Some("second"));

    repo.push(&mut ws1).expect("push");

    let mut conflict_ws = match repo.try_push(&mut ws2).expect("push") {
        Some(ws) => ws,
        _ => panic!("expected conflict"),
    };

    conflict_ws.merge(&mut ws2).unwrap();

    repo.push(&mut conflict_ws).expect("push");
}

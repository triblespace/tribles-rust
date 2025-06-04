use tribles::prelude::*;
use tribles::repo::{memoryrepo::InMemoryRepo, RepoPushResult, Repository};

#[test]
fn push_and_merge_conflict_resolution() {
    let storage = InMemoryRepo::default();
    let mut repo = Repository::new(storage);
    let mut ws1 = repo.branch("main").expect("create branch");
    let branch_id = ws1.branch_id();
    let mut ws2 = repo.checkout(branch_id).expect("checkout");

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

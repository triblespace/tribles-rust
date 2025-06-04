use tribles::prelude::*;
use tribles::repo::{memoryrepo::InMemoryRepo, RepoPushResult, Repository};

#[test]
fn workspace_commit_updates_head() {
    let storage = InMemoryRepo::default();
    let mut repo = Repository::new(storage);
    let mut ws = repo.branch("main").expect("create branch");

    ws.commit(TribleSet::new(), Some("change"));

    match repo.push(&mut ws) {
        Ok(RepoPushResult::Success()) => {}
        Ok(_) | Err(_) => panic!("push failed"),
    }
}

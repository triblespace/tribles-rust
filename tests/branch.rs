use tribles::prelude::*;
use tribles::repo::{memoryrepo::MemoryRepo, Repository};

#[test]
fn repository_branch_creates_branch() {
    let mut storage = MemoryRepo::default();
    let mut repo = Repository::new(storage);
    let ws = repo.branch("main").expect("create branch");
    let branch_id = ws.branch_id();

    match repo.checkout(branch_id) {
        Ok(_) => {}
        Err(_) => panic!("checkout failed"),
    }
}

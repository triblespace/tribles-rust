use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{memoryrepo::MemoryRepo, Repository};

#[test]
fn workspace_commit_updates_head() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let mut ws = repo.branch("main").expect("create branch");

    ws.commit(TribleSet::new(), Some("change"));

    match repo.push(&mut ws) {
        Ok(None) => {}
        Ok(_) | Err(_) => panic!("push failed"),
    }
}

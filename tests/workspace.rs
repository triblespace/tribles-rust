use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{self, memoryrepo::MemoryRepo, RepoPushResult, Repository};

#[test]
fn workspace_commit_updates_head() {
    let mut storage = MemoryRepo::default();
    let signing_key = SigningKey::generate(&mut OsRng);
    let init_commit = repo::commit::commit(&signing_key, [], None, None);
    let init_handle = storage.put(init_commit).unwrap();
    let branch_key = SigningKey::generate(&mut OsRng);
    let mut repo = Repository::new(storage);
    let branch_id = repo.branch("main", init_handle, branch_key);

    let mut ws = match repo.checkout(branch_id) {
        Ok(ws) => ws,
        Err(_) => panic!("checkout failed"),
    };

    ws.commit(TribleSet::new(), Some("change"));

    match repo.push(&mut ws) {
        Ok(RepoPushResult::Success()) => {}
        Ok(_) | Err(_) => panic!("push failed"),
    }
}

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{self, Repository};

mod util;
use util::InMemoryRepo;

#[test]
fn repository_branch_creates_branch() {
    let mut storage = InMemoryRepo::default();
    let commit_key = SigningKey::generate(&mut OsRng);
    let commit_set = repo::commit::commit(&commit_key, [], None, None);
    let commit_handle = storage.put(commit_set).unwrap();

    let branch_key = SigningKey::generate(&mut OsRng);
    let mut repo = Repository::new(storage);
    let branch_id = repo.branch("main", commit_handle, branch_key);

    match repo.checkout(branch_id) {
        Ok(_) => {}
        Err(_) => panic!("checkout failed"),
    }
}

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::repo::{memoryrepo::MemoryRepo, Repository};

#[test]
fn repository_branch_creates_branch() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let ws = repo.branch("main").expect("create branch");
    let branch_id = ws.branch_id();

    match repo.pull(branch_id) {
        Ok(_) => {}
        Err(_) => panic!("pull failed"),
    }
}

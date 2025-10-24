use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use triblespace::prelude::*;
use triblespace::repo::commit;
use triblespace::repo::memoryrepo::MemoryRepo;
use triblespace::repo::Repository;

#[test]
fn branch_from_and_pull_with_key() {
    // prepare storage with an initial commit
    let mut store = MemoryRepo::default();
    let key = SigningKey::generate(&mut OsRng);
    let commit_set = commit::commit_metadata(&key, [], None, None);
    let initial = store.put(commit_set).unwrap();

    let mut repo = Repository::new(store, key.clone());
    let branch_id = repo
        .create_branch("feature", Some(initial))
        .expect("branch from");
    let mut ws = repo.pull(*branch_id).expect("pull");
    ws.commit(TribleSet::new(), Some("work"));
    repo.push(&mut ws).expect("push");

    // pull using a different key should succeed
    let other_key = SigningKey::generate(&mut OsRng);
    repo.pull_with_key(*branch_id, other_key).expect("pull");
}

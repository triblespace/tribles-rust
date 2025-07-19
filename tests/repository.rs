use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{commit, memoryrepo::MemoryRepo, LookupError, Repository};

#[test]
fn branch_from_and_pull_with_key() {
    // prepare storage with an initial commit
    let mut store = MemoryRepo::default();
    let key = SigningKey::generate(&mut OsRng);
    let commit_set = commit::commit(&key, [], None, None);
    let initial = store.put(commit_set).unwrap();

    let mut repo = Repository::new(store, key.clone());
    let mut ws = repo.branch_from("feature", initial).expect("branch from");
    ws.commit(TribleSet::new(), Some("work"));
    repo.push(&mut ws).expect("push");

    // pull using a different key should succeed
    let other_key = SigningKey::generate(&mut OsRng);
    let branch_id = ws.branch_id();
    repo.pull_with_key(branch_id, other_key).expect("pull");
}

#[test]
fn lookup_branch_id_by_name_found() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let ws = repo.branch("dev").expect("create branch");
    let id = ws.branch_id();

    assert_eq!(repo.branch_id_by_name("dev").unwrap(), Some(id));
}

#[test]
fn lookup_branch_id_by_name_conflict() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    repo.branch("dev").expect("create branch");
    repo.branch("dev").expect("create second");

    match repo.branch_id_by_name("dev") {
        Err(LookupError::NameConflict(ids)) => assert_eq!(ids.len(), 2),
        _ => panic!("expected NameConflict"),
    }
}

#[test]
fn lookup_branch_id_by_name_missing() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));

    assert_eq!(repo.branch_id_by_name("nothing").unwrap(), None);
}

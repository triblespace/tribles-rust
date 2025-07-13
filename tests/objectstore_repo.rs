use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{self, objectstore::ObjectStoreRemote, Repository};
use tribles::value::schemas::hash::Blake3;
use url::Url;

#[test]
fn objectstore_branch_creates_branch() {
    let url = Url::parse("memory:///repo").unwrap();
    let storage = ObjectStoreRemote::<Blake3>::with_url(&url).unwrap();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let ws = repo.branch("main").expect("create branch");
    let branch_id = ws.branch_id();

    repo.checkout(branch_id).expect("checkout");
}

#[test]
fn objectstore_workspace_commit_updates_head() {
    let url = Url::parse("memory:///repo2").unwrap();
    let storage = ObjectStoreRemote::<Blake3>::with_url(&url).unwrap();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let mut ws = repo.branch("main").expect("create branch");

    ws.commit(TribleSet::new(), Some("change"));

    match repo.push(&mut ws).expect("push") {
        None => {}
        _ => panic!("push failed"),
    }
}

#[test]
fn objectstore_branch_from_and_checkout_with_key() {
    let url = Url::parse("memory:///repo3").unwrap();
    let mut store = ObjectStoreRemote::<Blake3>::with_url(&url).unwrap();
    let key = SigningKey::generate(&mut OsRng);
    let commit_set = repo::commit::commit(&key, [], None, None);
    let initial = store.put(commit_set).unwrap();

    let mut repo = Repository::new(store, key.clone());
    let mut ws = repo.branch_from("feature", initial).expect("branch from");
    ws.commit(TribleSet::new(), Some("work"));
    repo.push(&mut ws).expect("push");

    let other_key = SigningKey::generate(&mut OsRng);
    let branch_id = ws.branch_id();
    repo.checkout_with_key(branch_id, other_key)
        .expect("checkout");
}

#[test]
fn objectstore_push_and_merge_conflict_resolution() {
    let url = Url::parse("memory:///repo4").unwrap();
    let storage = ObjectStoreRemote::<Blake3>::with_url(&url).unwrap();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let mut ws1 = repo.branch("main").expect("create branch");
    let branch_id = ws1.branch_id();
    let mut ws2 = repo.checkout(branch_id).expect("checkout");

    ws1.commit(TribleSet::new(), Some("first"));
    ws2.commit(TribleSet::new(), Some("second"));

    match repo.push(&mut ws1).expect("push") {
        None => {}
        _ => panic!("expected success"),
    }

    let mut conflict_ws = match repo.push(&mut ws2).expect("push") {
        Some(ws) => ws,
        _ => panic!("expected conflict"),
    };

    conflict_ws.merge(&mut ws2).unwrap();

    match repo.push(&mut conflict_ws).expect("push") {
        None => {}
        _ => panic!("expected success after merge"),
    }
}

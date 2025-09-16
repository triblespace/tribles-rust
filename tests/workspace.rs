use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::ancestors;
use tribles::repo::history_of;
use tribles::repo::memoryrepo::MemoryRepo;
use tribles::repo::nth_ancestor;
use tribles::repo::parents;
use tribles::repo::symmetric_diff;
use tribles::repo::Repository;

#[test]
fn workspace_commit_updates_head() {
    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    ws.commit(TribleSet::new(), Some("change"));

    match repo.push(&mut ws) {
        Ok(None) => {}
        Ok(_) | Err(_) => panic!("push failed"),
    }
}

#[test]
fn workspace_checkout_unions_commits() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let e1 = ufoid();
    let a1 = ufoid();
    let v1: Value<R256> = 1i128.to_value();
    let t1 = Trible::new(&e1, &a1, &v1);
    let mut s1 = TribleSet::new();
    s1.insert(&t1);

    ws.commit(s1.clone(), None);
    let c1 = ws.head().unwrap();

    let e2 = ufoid();
    let a2 = ufoid();
    let v2: Value<R256> = 2i128.to_value();
    let t2 = Trible::new(&e2, &a2, &v2);
    let mut s2 = TribleSet::new();
    s2.insert(&t2);

    ws.commit(s2.clone(), None);
    let c2 = ws.head().unwrap();

    let result = ws.checkout(&[c1, c2][..]).expect("checkout");

    let mut expected = s1;
    expected.union(s2);

    assert_eq!(result, expected);
}

#[test]
fn workspace_checkout_single_commit() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let e = ufoid();
    let a = ufoid();
    let v: Value<R256> = 42i128.to_value();
    let t = Trible::new(&e, &a, &v);
    let mut s = TribleSet::new();
    s.insert(&t);

    ws.commit(s.clone(), None);
    let c = ws.head().unwrap();

    let result = ws.checkout(c).expect("checkout single");

    assert_eq!(result, s);
}

#[test]
fn workspace_checkout_vec_commits() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let mut sets = Vec::new();
    let mut commits = Vec::new();
    for i in 0..3i128 {
        let e = ufoid();
        let a = ufoid();
        let v: Value<R256> = i.to_value();
        let t = Trible::new(&e, &a, &v);
        let mut s = TribleSet::new();
        s.insert(&t);
        ws.commit(s.clone(), None);
        sets.push(s);
        commits.push(ws.head().unwrap());
    }

    let result = ws.checkout(commits.clone()).expect("checkout vec");

    let mut expected = TribleSet::new();
    for s in sets {
        expected.union(s);
    }

    assert_eq!(result, expected);
}

#[test]
fn workspace_checkout_range_variants() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let mut sets = Vec::new();
    let mut handles = Vec::new();
    for i in 0..3i128 {
        let e = ufoid();
        let a = ufoid();
        let v: Value<R256> = i.to_value();
        let t = Trible::new(&e, &a, &v);
        let mut s = TribleSet::new();
        s.insert(&t);
        ws.commit(s.clone(), None);
        sets.push(s);
        handles.push(ws.head().unwrap());
    }

    let (c1, c2, c3) = (handles[0], handles[1], handles[2]);

    let mut s1s2 = sets[0].clone();
    s1s2.union(sets[1].clone());
    let mut s2s3 = sets[1].clone();
    s2s3.union(sets[2].clone());
    let mut s1s2s3 = s1s2.clone();
    s1s2s3.union(sets[2].clone());

    assert_eq!(ws.checkout(c1..c3).unwrap(), s2s3.clone());
    assert_eq!(ws.checkout(c2..).unwrap(), sets[2].clone());
    assert_eq!(ws.checkout(..c3).unwrap(), s1s2s3.clone());
    assert_eq!(ws.checkout(..).unwrap(), s1s2s3);
}

#[test]
fn workspace_checkout_symmetric_diff() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let mut sets = Vec::new();
    let mut handles = Vec::new();
    for i in 0..3i128 {
        let e = ufoid();
        let a = ufoid();
        let v: Value<R256> = i.to_value();
        let t = Trible::new(&e, &a, &v);
        let mut s = TribleSet::new();
        s.insert(&t);
        ws.commit(s.clone(), None);
        sets.push(s);
        handles.push(ws.head().unwrap());
    }

    let (c1, _c2, c3) = (handles[0], handles[1], handles[2]);
    let mut expected = sets[1].clone();
    expected.union(sets[2].clone());

    assert_eq!(ws.checkout(symmetric_diff(c1, c3)).unwrap(), expected);
}

#[test]
fn workspace_get_local_and_base() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let e = ufoid();
    let a = ufoid();
    let v: Value<R256> = 123i128.to_value();
    let t = Trible::new(&e, &a, &v);
    let mut set = TribleSet::new();
    set.insert(&t);

    let handle = ws.put(set.clone());
    ws.commit(set.clone(), None);

    let local: TribleSet = ws.get(handle).expect("get local");
    assert_eq!(local, set);

    repo.push(&mut ws).expect("push");
    let branch_id = ws.branch_id();
    let mut ws2 = repo.pull(branch_id).expect("pull");

    let base: TribleSet = ws2.get(handle).expect("get base");
    assert_eq!(base, set);
}

#[test]
fn workspace_checkout_head_collects_history() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let mut sets = Vec::new();
    for i in 0..3i128 {
        let e = ufoid();
        let a = ufoid();
        let v: Value<R256> = i.to_value();
        let t = Trible::new(&e, &a, &v);
        let mut s = TribleSet::new();
        s.insert(&t);
        ws.commit(s.clone(), None);
        sets.push(s);
    }

    let head = ws.head().unwrap();
    let result = ws.checkout(ancestors(head)).expect("checkout history");

    let mut expected = sets[0].clone();
    expected.union(sets[1].clone());
    expected.union(sets[2].clone());

    assert_eq!(result, expected);
}

#[test]
fn workspace_nth_ancestor_selector() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let mut sets = Vec::new();
    for i in 0..3i128 {
        let e = ufoid();
        let a = ufoid();
        let v: Value<R256> = i.to_value();
        let t = Trible::new(&e, &a, &v);
        let mut s = TribleSet::new();
        s.insert(&t);
        ws.commit(s.clone(), None);
        sets.push(s);
    }

    let head = ws.head().unwrap();

    let result = ws
        .checkout(nth_ancestor(head, 2))
        .expect("checkout nth ancestor");
    assert_eq!(result, sets[0]);

    let empty = ws
        .checkout(nth_ancestor(head, 3))
        .expect("checkout past root");
    assert_eq!(empty.len(), 0);
}

#[test]
fn workspace_parents_selector() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));

    // Base commit so both workspaces share a common ancestor.
    let main_branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws_main = repo.pull(*main_branch_id).expect("pull branch state");
    let e0 = ufoid();
    let a0 = ufoid();
    let v0: Value<R256> = 0i128.to_value();
    let t0 = Trible::new(&e0, &a0, &v0);
    let mut s0 = TribleSet::new();
    s0.insert(&t0);
    ws_main.commit(s0, None);
    repo.push(&mut ws_main).expect("push base");

    // Fork a second workspace from the same base commit.
    let mut ws_feature = repo.pull(ws_main.branch_id()).expect("pull branch state");

    // Divergent commits on both workspaces.
    let e1 = ufoid();
    let a1 = ufoid();
    let v1: Value<R256> = 1i128.to_value();
    let t1 = Trible::new(&e1, &a1, &v1);
    let mut s1 = TribleSet::new();
    s1.insert(&t1);
    ws_main.commit(s1.clone(), None);

    let e2 = ufoid();
    let a2 = ufoid();
    let v2: Value<R256> = 2i128.to_value();
    let t2 = Trible::new(&e2, &a2, &v2);
    let mut s2 = TribleSet::new();
    s2.insert(&t2);
    ws_feature.commit(s2.clone(), None);

    // Merge the feature workspace into main to create a commit with two parents.
    ws_main.merge(&mut ws_feature).expect("merge workspaces");
    let merge_commit = ws_main.head().expect("merge head");

    let result = ws_main
        .checkout(parents(merge_commit))
        .expect("checkout parents");

    let mut expected = s1;
    expected.union(s2);

    assert_eq!(result, expected);
}

#[test]
fn workspace_history_of_entity() {
    use tribles::value::schemas::r256::R256;

    let storage = MemoryRepo::default();
    let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None).expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull");

    let entity = ufoid();
    let a1 = ufoid();
    let a2 = ufoid();
    let v1: Value<R256> = 1i128.to_value();
    let v2: Value<R256> = 2i128.to_value();

    let mut s1 = TribleSet::new();
    s1.insert(&Trible::new(&entity, &a1, &v1));
    ws.commit(s1.clone(), None);

    let mut s2 = TribleSet::new();
    s2.insert(&Trible::new(&ufoid(), &a1, &v1));
    ws.commit(s2.clone(), None);

    let mut s3 = TribleSet::new();
    s3.insert(&Trible::new(&entity, &a2, &v2));
    ws.commit(s3.clone(), None);

    let result = ws.checkout(history_of(*entity)).expect("history_of");

    let mut expected = s1;
    expected.union(s3);

    assert_eq!(result, expected);
}

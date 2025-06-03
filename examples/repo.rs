use ed25519_dalek::SigningKey;
use tribles::examples::literature;
use tribles::prelude::*;
use tribles::repo::{commit::commit, RepoPushResult, Repository};

fn main() {
    const MAX_PILE_SIZE: usize = 1 << 20;
    let tmp = tempfile::tempdir().expect("tmp dir");
    let path = tmp.path().join("repo.pile");

    // Create a local pile to store blobs and branches
    let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).expect("open pile");

    let key = SigningKey::generate(&mut rand::rngs::OsRng);

    // Create an initial commit that just stores an empty dataset
    let init_blob = TribleSet::new().to_blob();
    let init_commit_set = commit(&key, [], Some("init"), Some(init_blob.clone()));
    pile.put(init_blob).expect("store blob");
    let init_commit = pile.put(init_commit_set.to_blob()).expect("store commit");



    // Create a repository from the pile and initialize the main branch
    let mut repo = Repository::new(pile);
    let branch_id = repo.branch("main", init_commit, key.clone());

    // First workspace adds Alice and pushes
    let mut ws1 = repo.checkout(branch_id).expect("checkout");
    let change = literature::entity!(&ufoid(), { firstname: "Alice" });
    ws1.commit(change, Some("add alice"));
    repo.push(&mut ws1).expect("push ws1");

    // Second workspace adds Bob and attempts to push, merging on conflict
    let mut ws2 = repo.checkout(branch_id).expect("checkout");
    let mut change = TribleSet::new();
    change += literature::entity!(&ufoid(), { firstname: "Bob" });
    ws2.commit(change, Some("add bob"));

    match repo.push(&mut ws2).expect("push ws2") {
            RepoPushResult::Success() => println!("Push ws2 succeeded"),
            RepoPushResult::Conflict(mut other) => {
                loop {
                    other.merge(&mut ws2).expect("merge");
                    match repo.push(&mut other).expect("push conflict") {
                        RepoPushResult::Success() => break,
                        RepoPushResult::Conflict(next) => other = next,
                    }
            }
        }
    }
}

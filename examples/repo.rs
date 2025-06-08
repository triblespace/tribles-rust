use tribles::examples::literature;
use tribles::prelude::*;
use tribles::repo::{RepoPushResult, Repository};

fn main() {
    const MAX_PILE_SIZE: usize = 1 << 20;
    let tmp = tempfile::tempdir().expect("tmp dir");
    let path = tmp.path().join("repo.pile");

    // Create a local pile to store blobs and branches
    let mut pile: Pile<MAX_PILE_SIZE> = Pile::open(&path).expect("open pile");

    // Create a repository from the pile and initialize the main branch
    let mut repo = Repository::new(pile);
    let mut ws1 = repo.branch("main").expect("create branch");
    let branch_id = ws1.branch_id();

    // First workspace adds Alice and pushes
    let mut change = TribleSet::new();
    change += literature::entity!(&ufoid(), { firstname: "Alice" });

    ws1.commit(change, Some("add alice"));
    repo.push(&mut ws1).expect("push ws1");

    // Second workspace adds Bob and attempts to push, merging on conflict
    let mut ws2 = repo.checkout(branch_id).expect("checkout");
    let mut change = TribleSet::new();
    change += literature::entity!(&ufoid(), { firstname: "Bob" });
    ws2.commit(change, Some("add bob"));

    match repo.push(&mut ws2).expect("push ws2") {
        RepoPushResult::Success() => println!("Push ws2 succeeded"),
        RepoPushResult::Conflict(mut other) => loop {
            other.merge(&mut ws2).expect("merge");
            match repo.push(&mut other).expect("push conflict") {
                RepoPushResult::Success() => break,
                RepoPushResult::Conflict(next) => other = next,
            }
        },
    }
}

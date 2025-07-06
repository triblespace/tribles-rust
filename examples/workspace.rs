use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{memoryrepo::MemoryRepo, RepoPushResult, Repository};

fn main() {
    let mut repo = Repository::new(MemoryRepo::default(), SigningKey::generate(&mut OsRng));

    // create a new branch and add a commit
    let mut ws = repo.branch("feature").expect("create branch");
    ws.commit(TribleSet::new(), Some("start feature work"));

    // push, merging on conflict
    while let RepoPushResult::Conflict(mut other) = repo.push(&mut ws).expect("push") {
        ws.merge(&mut other).expect("merge");
    }
    println!("pushed");
}

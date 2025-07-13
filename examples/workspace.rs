use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{memoryrepo::MemoryRepo, Repository};

fn main() {
    let mut repo = Repository::new(MemoryRepo::default(), SigningKey::generate(&mut OsRng));

    // create a new branch and add a commit
    let mut workspace = repo.branch("feature").expect("create branch");
    workspace.commit(TribleSet::new(), Some("start feature work"));

    // attempt to push, merging conflicts before retrying
    while let Some(mut incoming) = repo.push(&mut workspace).expect("push") {
        // merge our local changes into the conflicting workspace
        incoming.merge(&mut workspace).expect("merge");
        // push the merged workspace on the next iteration
        workspace = incoming;
    }
    println!("pushed");
}

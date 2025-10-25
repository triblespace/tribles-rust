use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use triblespace::prelude::*;
use triblespace::repo::memoryrepo::MemoryRepo;
use triblespace::repo::Repository;

fn main() {
    let mut repo = Repository::new(MemoryRepo::default(), SigningKey::generate(&mut OsRng));

    // create a new branch and add a commit
    let branch_id = repo.create_branch("feature", None).expect("create branch");
    let mut workspace = repo.pull(*branch_id).expect("pull");
    workspace.commit(TribleSet::new(), Some("start feature work"));

    // attempt to push, merging conflicts before retrying
    while let Some(mut incoming) = repo.try_push(&mut workspace).expect("push") {
        // merge our local changes into the conflicting workspace
        incoming.merge(&mut workspace).expect("merge");
        // push the merged workspace on the next iteration
        workspace = incoming;
    }
    println!("pushed");
}

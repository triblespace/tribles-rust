# Repository and Workspace API

The repository API offers a lightweight version control mechanism for
trible data. A repository is backed by a pair of blob and branch stores
that persist commits and branch metadata.
Work is performed in a `Workspace` that tracks a branch head locally
until the changes are pushed back to the repository.

## Basic usage

```rust
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::{memoryrepo::MemoryRepo, RepoPushResult, Repository};

let storage = MemoryRepo::default();
let mut repo = Repository::new(storage, SigningKey::generate(&mut OsRng));
let mut ws = repo.branch("main").expect("create branch");

NS! {
    pub namespace literature {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
    }
}
let author = fucid();
ws.commit(
    literature::entity!(&author, {
        firstname: "Frank",
        lastname: "Herbert",
    }),
    Some("initial commit"),
);

match repo.push(&mut ws).expect("push") {
    RepoPushResult::Success() => {}
    RepoPushResult::Conflict(_) => panic!("unexpected conflict"),
}
```

`checkout` creates a new workspace from an existing branch while
`branch_from` can be used to start a new branch from a specific commit
handle. See `examples/workspace.rs` for a more complete example.

### Handling conflicts

`push` may return `RepoPushResult::Conflict` when the branch has moved on
the repository. The returned workspace contains the updated branch
metadata and must be pushed after merging your changes:

```rust
while let RepoPushResult::Conflict(mut other) = repo.push(&mut ws)? {
    other.merge(&mut ws)?;
    ws = other;
}
```

`push` performs a compare‐and‐swap (CAS) update on the branch metadata.
This optimistic concurrency control keeps branches consistent without
locking and can be emulated by many storage systems (for example by
using conditional writes on S3).

## Git parallels

The API deliberately mirrors concepts from Git to make its usage familiar:

- A `Repository` stores commits and branch metadata similar to a remote.
- `Workspace` is akin to a working directory combined with an index. It
  tracks changes against a branch head until you `push` them.
- `branch` and `branch_from` correspond to creating new branches from the
  current head or from a specific commit, respectively.
- `push` updates the repository atomically. If the branch advanced in the
  meantime, you receive a conflict workspace which can be merged before
  retrying the push.
- `checkout` is similar to cloning a branch into a new workspace.

`checkout` uses the repository's default signing key for new commits. If you
need to work with a different identity, the `_with_key` variants allow providing
an explicit key when branching or checking out.

These parallels should help readers leverage their Git knowledge when
working with trible repositories.

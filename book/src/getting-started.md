# Getting Started

First add the required crates to your project:

```bash
cargo add tribles ed25519-dalek rand
```

This example uses `ed25519-dalek` to generate a signing key and `rand` for randomness.

Next create a simple repository, initialize a `main` branch, and commit some
data. The `tribles::prelude` module re-exports the `attributes!` and `entity!`
macros, so you can mint the attributes your application needs right alongside
the quick-start example:

```rust
use tribles::prelude::*;
use tribles::repo::Repository;
use std::path::Path;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

mod getting_started {
    use tribles::prelude::*;
    use tribles::prelude::valueschemas::*;

    attributes! {
        // Pick a unique 32-character hex id for each attribute you define.
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as firstname: ShortString;
    }
}

fn main() {
#     let _ = std::fs::remove_file("example.pile");
    let mut pile = Pile::open(Path::new("example.pile")).expect("open pile");
    pile.restore().expect("restore pile");
    let mut repo = Repository::new(pile, SigningKey::generate(&mut OsRng));

    let branch_id = repo
        .create_branch("main", None)
        .expect("create branch");
    let mut ws = repo.pull(*branch_id).expect("pull branch");

    ws.commit(entity! { &ufoid() @ getting_started::firstname: "Alice" }, None);
    assert!(repo.push(&mut ws).expect("push branch").is_none());
    repo.close().expect("close repository");
#     std::fs::remove_file("example.pile").expect("remove example pile");
}
```

Running this program with `cargo run` creates an `example.pile` file in the current
directory and pushes a single entity to the `main` branch. `Repository::create_branch`
registers the branch and returns an `ExclusiveId` guard; pass its `Id`
to `Repository::pull` (via dereferencing or `ExclusiveId::release`) to obtain a
`Workspace` for writing commits.

When working with pile-backed repositories it is important to close them
explicitly once you are done so buffered data is flushed and any errors are
reported while you can still decide how to handle them. The `repo.close()?;`
call in the example surfaces those errors; if the repository were only dropped,
failures would have to be logged or panic instead. Alternatively, you can
recover the underlying pile with `Repository::into_storage` and call
`Pile::close()` yourself.

See the [crate documentation](https://docs.rs/tribles/latest/tribles/) for
additional modules and examples.

Note: the `pattern!` macro used in queries treats values prefixed with `?` as
variable bindings and more complex expressions (including string literals) as
literal values. Use `_?ident` when you want a fresh variable that is scoped to
the macro invocation without listing it in the query head.

## Switching signing identities

The setup above generates a single signing key for brevity, but collaborating
authors typically hold individual keys. Call `Repository::set_signing_key`
before branching or pulling when you need a different default identity, or use
`Repository::create_branch_with_key` and `Repository::pull_with_key` to choose a
specific key per branch or workspace. The [Managing signing identities](repository-workflows.html#managing-signing-identities)
section covers this workflow in more detail.

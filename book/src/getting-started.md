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
    std::fs::remove_file("example.pile").expect("remove example pile");
}
```

Running this program with `cargo run` pushes a single entity to the `main`
branch. The example removes the pile file at the end so repeated doc test runs
start fresh; comment out that line if you want to inspect the stored data.

See the [crate documentation](https://docs.rs/tribles/latest/tribles/) for
additional modules and examples.

Note: the `pattern!` macro used in queries treats a bare identifier as a
variable binding and more complex expressions (including string literals) as
literal values; parentheses may still be used to force a literal where desired.

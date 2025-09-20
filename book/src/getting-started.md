# Getting Started

First add the required crates to your project:

```bash
cargo add tribles ed25519-dalek rand
```

This example uses `ed25519-dalek` to generate a signing key and `rand` for randomness.

Next create a simple repository and commit some data:

```rust,ignore
use tribles::prelude::*;
use tribles::examples::literature;
use tribles::repo::Repository;
use std::path::Path;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pile = Pile::open(Path::new("example.pile"))?;
    pile.restore()?;
    let mut repo = Repository::new(pile, SigningKey::generate(&mut OsRng));
    let branch_id = repo.create_branch("main", None)?;
    let mut ws = repo.pull(*branch_id)?;

    ws.commit(crate::entity!{ &ufoid() @ literature::firstname: "Alice" }, None);
    repo.push(&mut ws)?;
    Ok(())
}
```

Running this program with `cargo run` creates an `example.pile` file in the current
directory and pushes a single entity to the `main` branch. `Repository::create_branch`
registers the branch and returns an `ExclusiveId` guard; pass its `Id`
to `Repository::pull` (via dereferencing or `ExclusiveId::release`) to obtain a
`Workspace` for writing commits.

See the [crate documentation](https://docs.rs/tribles/latest/tribles/) for
additional modules and examples.

Note: the `pattern!` macro used in queries treats a bare identifier as a
variable binding and more complex expressions (including string literals) as
literal values; parentheses may still be used to force a literal where desired.

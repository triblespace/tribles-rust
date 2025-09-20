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
    let mut ws = repo.branch("main")?;

    ws.commit(crate::entity!{ &ufoid() @ literature::firstname: "Alice" }, None);
    repo.push(&mut ws)?;
    Ok(())
}
```

Running this program with `cargo run` creates an `example.pile` file in the current
directory and pushes a single entity to the `main` branch.

See the [crate documentation](https://docs.rs/tribles/latest/tribles/) for
additional modules and examples.

Note: the `pattern!` macro used in queries treats a bare identifier as a
variable binding and more complex expressions (including string literals) as
literal values; parentheses may still be used to force a literal where desired.

## Switching signing identities

The setup above generates a single signing key for brevity, but collaborating
authors typically hold individual keys. Call `Repository::set_signing_key`
before branching or pulling when you need a different default identity, or use
`Repository::create_branch_with_key` and `Repository::pull_with_key` to choose a
specific key per branch or workspace. The [Managing signing identities](repository-workflows.html#managing-signing-identities)
section covers this workflow in more detail.

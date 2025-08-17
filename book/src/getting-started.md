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

const MAX_PILE_SIZE: usize = 1 << 20;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pile: Pile<MAX_PILE_SIZE> = Pile::open(Path::new("example.pile"))?;
    let mut repo = Repository::new(pile, SigningKey::generate(&mut OsRng));
    let mut ws = repo.branch("main")?;

    ws.commit(literature::entity!(&ufoid(), { firstname: "Alice" }), None);
    repo.push(&mut ws)?;
    Ok(())
}
```

Running this program with `cargo run` creates an `example.pile` file in the current
directory and pushes a single entity to the `main` branch.

See the [crate documentation](https://docs.rs/tribles/latest/tribles/) for
additional modules and examples.

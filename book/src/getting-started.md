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
    repo.close()?;
    Ok(())
}
```

Running this program with `cargo run` creates an `example.pile` file in the current
directory and pushes a single entity to the `main` branch.

When working with pile-backed repositories it is important to close them
explicitly once you are done so buffered data is flushed and any errors are
reported while you can still decide how to handle them. The `repo.close()?;`
call in the example surfaces those errors; if the repository were only dropped,
failures would have to be logged or panic instead. Alternatively, you can
recover the underlying pile with `Repository::into_storage` and call
`Pile::close()` yourself.

See the [crate documentation](https://docs.rs/tribles/latest/tribles/) for
additional modules and examples.

Note: the `pattern!` macro used in queries treats a bare identifier as a
variable binding and more complex expressions (including string literals) as
literal values; parentheses may still be used to force a literal where desired.

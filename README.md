![Crates.io Version](https://img.shields.io/crates/v/tribles)
![docs.rs](https://img.shields.io/docsrs/tribles)
![Discord Shield](https://discordapp.com/api/guilds/795317845181464651/widget.png?style=shield)

🚧🚧🚧 Please note that this is work in progress, so while a lot of things have settled by now, we still favour breaking backwards compatiblity for seeminly minor improvements. 🚧🚧🚧


<img src="https://github.com/triblespace/tribles-rust/blob/main/sticker.png?raw=true" width="300"
 alt="The mascot of the trible.space a cute fluffy trible with three eyes."/>


# About

> “Insufficient facts always invite danger.”  
> — *Mr. Spock*

**Trible Space** is a data space and knowledge graph standard. It offers metadata management capabilities similar to file- and version-control systems, combined with the queryability and convenience of an embedded database, tailored towards use with simple blob storage. It is designed to be a holistic yet lightweight data storage solution that can be used in a variety of contexts, from embedded systems to distributed cloud services.

Our goal is to re-invent data storage from first principles and overcome the shortcomings of prior "Semantic Web"/triple-store technologies. By focusing on simplicity, canonical data formats, cryptographic identifiers, and clean distributed semantics, we aim to provide a lean, lightweight yet powerful toolkit for knowledge representation, database management, and data exchange use cases.

## Features

- **Lean, Lightweight & Flexible**: Data storage seamlessly scales from in-memory data organization to large-scale blob and metadata storage on S3 like services.
- **Distributed**: Eventually consistent CRDT semantics (based on the CALM principle), compressed zero-copy archives, and built-in version control.
- **Predictable Performance**: An optimizer-free design using novel algorithms and data structures removes the need for manual query-tuning and enables single-digit microsecond latency.  
- **Fast In-Memory Datasets**: Enjoy cheap copy-on-write (COW) semantics and speedy set operations, allowing you to treat entire datasets as values.
- **Compile-Time Typed Queries**: Automatic type inference, type-checking, and auto-completion make writing queries a breeze. You can even create queries that span multiple datasets and native Rust data structures.
- **Low Overall Complexity**: We aim for a design that feels obvious (in the best way) and makes good use of existing language facilities. A serverless design makes it completely self-sufficient for local use and requires only an S3-compatible service for distribution.
- **Easy Implementation**: The spec is designed to be friendly to high- and low-level languages, or even hardware implementations.
- **Lock-Free Blob Writes**: Blob data is appended with a single `O_APPEND` write. Each handle advances an in-memory `applied_length` only if no other writer has appended in between, scanning any gap to ingest missing records. Concurrent writers may duplicate blobs, but hashes guarantee consistency. Updating branch heads uses a short `flush → refresh → lock → refresh → append → unlock` sequence.
- **Coordinated Refresh**: `refresh` acquires a shared file lock while scanning to avoid races with `restore` truncating the pile.

# Community

If you have any questions or want to chat about graph databases hop into our [discord](https://discord.gg/v7AezPywZS).

## Getting Started

Add the crate to your project:

```bash
cargo add tribles
```

Then create a pile-backed repository and commit some data. The snippet below uses the `literature` example namespace which is expanded further down in the [Example](#example) section.

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

    ws.commit(crate::entity!(&ufoid(), { literature::firstname: "Alice" }), None);
    repo.push(&mut ws)?;
    Ok(())
}
```

# Example

```rust
use tribles::prelude::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::blobschemas::*;

NS! {
    pub namespace literature {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
        "FCCE870BECA333D059D5CD68C43B98F0" as page_count: R256;
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
    }
}

fn main() -> std::io::Result<()> {
    let mut blobs = MemoryBlobStore::new();
    let mut set = TribleSet::new();

    let author_id = ufoid();

    // Note how the entity macro returns TribleSets that can be cheaply merged
    // into our existing dataset.
    set += crate::entity!(&author_id, {
                literature::firstname: "Frank",
                literature::lastname: "Herbert",
            });

    set += crate::entity!(&author_id, {
                literature::title: "Dune",
                literature::author: &author_id,
                literature::quote: blobs.put("Deep in the human unconscious is a \
                pervasive need for a logical universe that makes sense. \
                But the real universe is always one step beyond logic.").unwrap(),
                literature::quote: blobs.put("I must not fear. Fear is the \
                mind-killer. Fear is the little-death that brings total \
                obliteration. I will face my fear. I will permit it to \
                pass over me and through me. And when it has gone past I \
                will turn the inner eye to see its path. Where the fear \
                has gone there will be nothing. Only I will remain.").unwrap(),
            });

    let title = "Dune";

    // We can then find all entities matching a certain pattern in our dataset.
    for (_, f, l, q) in find!(
        (author: (), first: String, last: Value<_>, quote),
        crate::pattern!(&set, [
            { author @
                literature::firstname: first,
                literature::lastname: last
            },
            {
                literature::title: (title),
                literature::author: author,
                literature::quote: quote
            }])) {
        let q: View<str> = blobs.reader().unwrap().get(q).unwrap();
        let q = q.as_ref();

        println!("'{q}'\n - from {title} by {f} {}.", l.from_value::<&str>())
    }
    Ok(())
}
```

## Tribles Book

For a step-by-step narrative guide, see the [Tribles Book](book/README.md).
To build the HTML locally, first install `mdbook` with `cargo install mdbook`
and then run:

```bash
./scripts/build_book.sh
```

For details on setting up a development environment, see [Developing Locally](book/src/contributing.md).

# Learn More

The best way to get started is to read the [Tribles Book](https://triblespace.github.io/tribles-rust/). The following links mirror the book's chapter order so you can progress from the basics to more advanced topics:

1. [Introduction](https://triblespace.github.io/tribles-rust/introduction.html)
2. [Getting Started](https://triblespace.github.io/tribles-rust/getting-started.html)
3. [Architecture](https://triblespace.github.io/tribles-rust/architecture.html)
4. [Query Language](https://triblespace.github.io/tribles-rust/query-language.html)
5. [Incremental Queries](https://triblespace.github.io/tribles-rust/incremental-queries.html)
6. [Schemas](https://triblespace.github.io/tribles-rust/schemas.html)
7. [Repository Workflows](https://triblespace.github.io/tribles-rust/repository-workflows.html)
8. [Commit Selectors](https://triblespace.github.io/tribles-rust/commit-selectors.html)
9. [Philosophy](https://triblespace.github.io/tribles-rust/deep-dive/philosophy.html)
10. [Identifiers](https://triblespace.github.io/tribles-rust/deep-dive/identifiers.html)
11. [Trible Structure](https://triblespace.github.io/tribles-rust/deep-dive/trible-structure.html)
12. [Pile Format](https://triblespace.github.io/tribles-rust/pile-format.html)
## License

Licensed under either of

* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

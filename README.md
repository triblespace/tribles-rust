![Crates.io Version](https://img.shields.io/crates/v/tribles)
![docs.rs](https://img.shields.io/docsrs/tribles)
![Discord Shield](https://discordapp.com/api/guilds/795317845181464651/widget.png?style=shield)

🚧🚧🚧 Please note that this is work in progress, so while a lot of things have settled by now, we still favour breaking backwards compatiblity for seeminly minor improvements. 🚧🚧🚧


<img src="https://github.com/triblespace/tribles-rust/blob/master/sticker.png?raw=true" width="300"
 alt="The mascot of the trible.space a cute fluffy trible with three eyes."/>


# About

> “Insufficient facts always invite danger.”  
> — *Mr. Spock*

**Trible Space** is a data space and knowledge graph standard. It offers metadata management capabilities similar to file- and version-control systems, combined with the queryability and convenience of an embedded database, tailored towards use with simple blob storage. It is designed to be a holistic yet lightweight data storage solution that can be used in a variety of contexts, from embedded systems to distributed cloud services.

Our goal is to re-invent data storage from first principles and overcome the shortcomings of prior "Semantic Web"/triple-store technologies. By focusing on simplicity, canonical data formats, cryptographic identifiers, and clean distributed semantics, we aim to provide a lean, lightweight yet powerful toolkit for knowledge representation, database management, and data exchange use cases.

## Features

- **Lean, Lightweight & Flexible**: Data storage seamlessly scales from in-memory data organization to large-scale blob and metadata storage on S3 like services.
- **Distributed**: Eventually consistent CRDT semantics (based on the CALM principle), compressed zero-copy archives (WIP), and built-in version control.
- **Predictable Performance**: An optimizer-free design using novel algorithms and data structures removes the need for manual query-tuning and enables single-digit microsecond latency.  
- **Fast In-Memory Datasets**: Enjoy cheap copy-on-write (COW) semantics and speedy set operations, allowing you to treat entire datasets as values.
- **Compile-Time Typed Queries**: Automatic type inference, type-checking, and auto-completion make writing queries a breeze. You can even create queries that span multiple datasets and native Rust data structures.
- **Low Overall Complexity**: We aim for a design that feels obvious (in the best way) and makes good use of existing language facilities. A serverless design makes it completely self-sufficient for local use and requires only an S3-compatible service for distribution.
- **Easy Implementation**: The spec is designed to be friendly to high- and low-level languages, or even hardware implementations.

# Community

If you have any questions or want to chat about graph databases hop into our [discord](https://discord.gg/v7AezPywZS).

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
    set += literature::entity!(&author_id, {
                firstname: "Frank",
                lastname: "Herbert",
            });

    set += literature::entity!({
                title: "Dune",
                author: &author_id,
                quote: blobs.put("Deep in the human unconscious is a \
                pervasive need for a logical universe that makes sense. \
                But the real universe is always one step beyond logic.").unwrap(),
                quote: blobs.put("I must not fear. Fear is the \
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
        literature::pattern!(&set, [
            { author @
                firstname: first,
                lastname: last
            },
            {
                title: (title),
                author: author,
                quote: quote
            }])) {
        let q: View<str> = blobs.reader().get(q).unwrap();
        let q = q.as_ref();

        println!("'{q}'\n - from {title} by {f} {}.", l.from_value::<&str>())
    }
    Ok(())
}
```

# Getting Started

The best way to get started is to read the module documentation of the `tribles` crate. The following links provide an overview of the most important modules, in an order where you can start with the most basic concepts and work your way up to more advanced topics:

1. [Prelude](https://docs.rs/tribles/latest/tribles/prelude/index.html)
2. [Identifiers](https://docs.rs/tribles/latest/tribles/id/index.html)
3. [Values](https://docs.rs/tribles/latest/tribles/value/index.html)
4. [Blobs](https://docs.rs/tribles/latest/tribles/blob/index.html)
5. [Tribles](https://docs.rs/tribles/latest/tribles/trible/index.html)
6. [Namespaces](https://docs.rs/tribles/latest/tribles/namespace/index.html)
7. [Queries](https://docs.rs/tribles/latest/tribles/query/index.html)
8. [Remotes](https://docs.rs/tribles/latest/tribles/remote/index.html)
9. [Predefined Value Schemas](https://docs.rs/tribles/latest/tribles/value/schemas/index.html)
10. [Predefined Blob Schemas](https://docs.rs/tribles/latest/tribles/blob/schemas/index.html)
11. [Pile format and recovery](docs/pile.md)
## License

Licensed under either of

* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

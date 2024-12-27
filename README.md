![Crates.io Version](https://img.shields.io/crates/v/tribles)
![docs.rs](https://img.shields.io/docsrs/tribles)
![Discord Shield](https://discordapp.com/api/guilds/795317845181464651/widget.png?style=shield)

🚧🚧🚧 Please note that this is work in progress, so while a lot of things have settled by now, we still favour breaking backwards compatiblity for seeminly minor improvements. 🚧🚧🚧

# About

> “Insufficient facts always invite danger.”  
> — *Mr. Spock*

**Trible Space** is a data space and knowledge graph standard. It offers metadata management capabilities similar to file- and version-control systems, combined with the queryability and convenience of an embedded database, tailored towards use with simple blob storage. It is designed to be a lightweight, self-contained, and distributed data storage solution that can be used in a variety of contexts, from embedded systems to cloud services.

Our goal is to re-invent data storage from first principles and overcome the shortcomings of prior semantic web/triple-store technologies. By focusing on simplicity, canonical data formats, cryptographic identifiers, and clean distributed semantics, we aim to provide a lightweight yet powerful toolkit for knowledge representation, database management, and data exchange use cases.

## Features

- **Lightweight & Flexible**: Data storage should seamlessly scale from in-memory data organization to large-scale blob and metadata storage on S3.
- **Distributed**: Eventually consistent CRDT semantics (based on the CALM principle), compressed zero-copy archives (WIP), and built-in version control.
- **Predictable Performance**: An optimizer-free design using novel algorithms and data structures removes the need for manual query-tuning and enables single-digit microsecond latency.  
- **Fast In-Memory Datasets**: Enjoy cheap copy-on-write (COW) semantics and speedy set operations, allowing you to treat entire datasets as values.
- **Compile-Time Typed Queries**: Safer data handling, plus handy features like delta queries.  
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
        "76AE5012877E09FF0EE0868FE9AA0343" as height: R256;
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
    }
}

fn main() -> std::io::Result<()> {
    let mut blobs = BlobSet::new();
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
                quote: blobs.insert("Deep in the human unconscious is a \
                pervasive need for a logical universe that makes sense. \
                But the real universe is always one step beyond logic."),
                quote: blobs.insert("I must not fear. Fear is the \
                mind-killer. Fear is the little-death that brings total \
                obliteration. I will face my fear. I will permit it to \
                pass over me and through me. And when it has gone past I \
                will turn the inner eye to see its path. Where the fear \
                has gone there will be nothing. Only I will remain.")
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
        let q: &str = blobs.get(q).unwrap();

        println!("'{q}'\n - from {title} by {f} {}.", l.from_value::<&str>())
    }
    Ok(())
}
```
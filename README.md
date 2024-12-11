![Crates.io Version](https://img.shields.io/crates/v/tribles)
![docs.rs](https://img.shields.io/docsrs/tribles)
![Discord Shield](https://discordapp.com/api/guilds/795317845181464651/widget.png?style=shield)

🚧🚧🚧 Please note that this is work in progress, so while a lot of things have settled by now, we still favour breaking backwards compatiblity for seeminly minor improvements. 🚧🚧🚧

# About

> The real tragedy would be if people forgot that you can have new ideas about programming models in the first place. <br/> - [Bret Victor](https://worrydream.com/dbx/)

The [trible.space](https://trible.space) is our answer to the question "what if we re-invented data storage from first principles".
It is a knowledge graph standard for blob storage that provides metadata management capabilities similar to file- and version-control-systems
with the queryability and convenience of an embedded database.

We hope to overcome the shortcomings of previous semantic web/triple-store technologies, through simplicity, easy canonicalization and cryptographic identifiers, clean distributed semantics and lightweight libraries empowered by idiomatic host-language capabilities.

By reifying most concepts and operations as first class citizens, we hope to provide a toolkit that can be flexibly combined to serve a variety of knowledge representation, database, and data exchange use cases.

# Differentiators

- A novel family of worst case optimal join algorithms combined with a series of tailored datastructures obviates manual query-tuning.
- Optimizer-free query engine design, providing predicatble performance and enabling single digit μs latency.
- Fast in-memory datasets with cheap COW semantics (i.e. persistent immutability).
- Fast set operations over in-memory datasets.
- Separation of names and identities.
- Explicit abstract datatypes and concrete layouts.
- Durable compressed fully queryable zero-copy archives, based on succinct datastructures.
- Self describing and documenting.
- Eventually consistent distributed semantics based on CRDTs and CALM,
providing build-in version control.
- 🚧 Delta-Queries between arbitrary datasets.
- Compile-time typed queries and dataset construction.
- Low overall complexity. If you feel that stuff is obvious, maybe a bit boring, and that you could have come up with it yourself, then we achieved our goal.

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
        "76AE5012877E09FF0EE0868FE9AA0343" as height: FR256;
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
    }
}

fn main() -> std::io::Result<()> {
    let mut blobs = BlobSet::new();
    let mut set = TribleSet::new();

    let author_id = ufoid();

    set.union(literature::entity!(&author_id, {
                firstname: "Frank",
                lastname: "Herbert",
            }));

    set.union(literature::entity!({
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
            }));

    let title = "Dune";
    for (_, f, l, q) in find!(ctx,
    (author: (), first: String, last: Value<_>, quote),
            literature::pattern!(ctx, &set, [
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
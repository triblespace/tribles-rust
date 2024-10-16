![Discord Shield](https://discordapp.com/api/guilds/795317845181464651/widget.png?style=shield)

# About

> The real tragedy would be if people forgot that you can have new ideas about programming models in the first place. - [Bret Victor](https://worrydream.com/dbx/)

The trible.space is our answer to the question "what if we re-invented data storage from first principles".
It is a knowledge graph standard for blob storage that provides metadata management capabilities similar to file- and version-control-systems
with the queryability and convenience of an embedded database.

We hope to overcome the shortcomings of previous semantic web/triple-store technologies, through simplicity, easy canonicalization leveraging cryptographic methods, clean distributed semantics and lightweight libraries empowered by individual host-language capabilities.

By reifying most concepts and operations as first class citizens, we hope to provide a toolkit that can be flexibly combined to serve a variety of knowledge representation, database, and data exchange use cases.

# Getting Started

```rust
use tribles;



let set = Tribleset::new();

```

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
- Delta-Queries between arbitrary datasets.
- Compile-time typed queries and dataset construction.
- Low overall complexity.

# Community

If you have any questions or want to chat about graph databases hop into our [discord](https://discord.gg/v7AezPywZS).

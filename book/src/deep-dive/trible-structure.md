The `trible` module contains the definition of the `Trible` struct, which is the fundamental unit of knowledge in the knowledge graph.
Instance of `Trible`s are stored in `TribleSet`s which index the trible in various ways, allowing for efficient querying and retrieval of data.

``` text
┌────────────────────────────64 byte───────────────────────────┐
┌──────────────┐┌──────────────┐┌──────────────────────────────┐
│  entity-id   ││ attribute-id ││        inlined value         │
└──────────────┘└──────────────┘└──────────────────────────────┘
└────16 byte───┘└────16 byte───┘└────────────32 byte───────────┘
─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶
```

On a high level, a trible is a triple consisting of an entity, an attribute,
and a value. The entity and attribute are both 128‑bit abstract extrinsic
identifiers as described in [crate::id], while the value is an arbitrary
256‑bit [crate::value::Value]. The width of the value is chosen so that it can
hold an entire intrinsic identifier, allowing larger payloads to be referenced
via blobs without inflating the inlined representation.

## Abstract identifiers

Entities and attributes are purely extrinsic; their identifiers do not encode
any meaning beyond uniqueness. An entity may accrue additional tribles over
time and attributes simply name relationships without prescribing a schema.
This keeps the format agnostic to external ontologies and minimises accidental
coupling between datasets.

The value slot can carry any 256‑bit payload. Its size is dictated by the need
to embed an intrinsic identifier for out‑of‑line data. When a fact exceeds this
space the value typically stores a blob handle pointing to the larger payload.

Tribles are stored as a contiguous 64‑byte array with the entity occupying the
first 16 bytes, the attribute the next 16 and the value the final 32 bytes. The
name "trible" is a portmanteau of *triple* and *byte* and is pronounced like
"tribble" from Star Trek – hence the project's mascot, Robert the tribble.

## Index permutations

`TribleSet`s index each fact under all six permutations of entity (E), attribute
(A) and value (V) so any combination of bound variables can be resolved
efficiently:

```text
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│ EAV │  │ EVA │  │ AEV │  │ AVE │  │ VEA │  │ VAE │
└──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘
   │        │        │        │        │        │
┌───────────────────────────────────────────────────────┐
│            order-specific inner nodes                 │
└───────────────────────────────────────────────────────┘ 
   │        │        │        │        │        │
   ▼        ▼        ▼        ▼        ▼        ▼

┌───────────────────────────────────────────────────────┐
│                   SHARED LEAVES                       │
│     single canonical E–A–V tribles used by all        │
└───────────────────────────────────────────────────────┘
```

Each permutation has its own inner nodes, but all six share leaf nodes
containing the 64‑byte trible. This avoids a naïve six‑fold memory cost while
still letting the query planner pick the most selective ordering, keeping joins
resistant to skew.

## Advantages

- A total order over tribles enables efficient storage and canonicalisation.
- Simple byte‑wise segmentation supports indexing and querying without an
  interning mechanism, keeping memory usage low and parallelisation easy while
  avoiding the need for garbage collection.
- Schemas describe the value portion directly, making serialisation and
  deserialisation straightforward.
- The fixed 64‑byte layout makes it easy to estimate the physical size of a
  dataset as a function of the number of tribles stored.
- The minimalistic design aims to minimise entropy while retaining collision
  resistance, making it likely that a similar format would emerge through
  convergent evolution and could serve as a universal data interchange format.

## Set operations and monotonic semantics

`TribleSet`s provide familiar set-theoretic helpers such as
[`TribleSet::union`](https://docs.rs/triblespace/latest/triblespace/trible/struct.TribleSet.html#method.union),
[`TribleSet::intersection`](https://docs.rs/triblespace/latest/triblespace/trible/struct.TribleSet.html#method.intersection)
and
[`TribleSet::difference`](https://docs.rs/triblespace/latest/triblespace/trible/struct.TribleSet.html#method.difference).
Each of these operations returns a new `TribleSet` view without modifying the
inputs, making it straightforward to merge datasets, locate their overlap or
identify the facts that still need to propagate between replicas while keeping
all sources intact.

This design reflects the crate's commitment to CALM-friendly, monotonic
semantics. New information can be added freely, but existing facts are never
destroyed. Consequently, `difference` is intended for comparing snapshots
(e.g. "which facts are present in the remote set that I have not yet
indexed?") rather than for destructive deletion. This keeps workflows
declarative and convergent: sets can be combined in any order without
introducing conflicts, and subtraction simply reports the gaps that remain to
be filled.

# Direction and Consistency

In other triple stores the direction of the edge drawn by a triple is often
choosen incidentally, e.g. there is no intrinsic preference for `hasColor` over
`colorOf`. This can lead to confusion and inconsistency in the graph, as
different writers might choose different directions for the same edge.
This is typically solved by:
- Automatically inferring the opposite edge for every edge inserted,
as done by OWL and RDF with the `inverseOf` predicate. Leading to a
doubling of the number of edges in the graph or inference at query time.
- Endless bikeshedding about the "right" direction of edges.

In the `triblespace` crate we solve this problem by giving the direction of the edge
an explicit semantic meaning: The direction of the edge indicates which entity
is the one making the statement, i.e. which entity is observing the fact
or proclaiming the relationship. This is a simple and consistent rule that
naturally fits into a distributed system, where each entity is associated with
a single writer that is responsible the consistency of the facts it asserts.
- see [ID Ownership](crate::id).

A different perspective is that edges are always ordered from describing
to described entities, with circles constituting consensus between them.

For example, the edge `hasColor` is always drawn from the entity that has
the color to the entity that represents the color. This makes the direction
of the edge a natural consequence of the semantics of the edge, and not
an arbitrary choice.

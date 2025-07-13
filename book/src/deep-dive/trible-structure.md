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

In the `tribles` crate we solve this problem by giving the direction of the edge
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

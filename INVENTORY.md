# Inventory

## Potential Removals
- None at the moment.

## Completed Work
- None yet. This file now tracks development ideas.

## Desired Functionality
- Finalize the compressed zero-copy archive format currently mentioned as WIP.
- Provide additional examples showcasing advanced queries and repository usage.
- Add incremental query support building on the union constraint so
  results can update when datasets change without full recomputation.
  Namespaces will expose a `delta!` operator similar to `pattern!`
  that receives the previous and current `TribleSet`, calls `union!`
  internally and matches only the newly added tribles. See the book's
  [Incremental Queries](book/src/incremental-queries.md) chapter for
  the planned approach.
- Explore replacing `CommitSelector` ranges with a set-based API
  built on commit reachability. The goal is to mirror git's revision
  selection semantics (similar to `rev-list` or `rev-parse`).
  Combinators like `union`, `intersection` and `difference` should let
  callers express queries such as "A minus B" or "ancestors of A
  intersect B". Commit sets themselves would be formed by primitives
  like `ancestors(<commit>)` and `descendants(<commit>)` so selectors
  map directly to the commit graph.
- Generate namespaces from a `TribleSet` description so tooling can
  derive them programmatically. Rewriting `pattern!` as a procedural
  macro will be the first step toward this automation.
- Benchmark PATCH performance across typical workloads.
- Investigate the theoretical complexity of PATCH operations.
- Measure practical space usage for PATCH with varying dataset sizes.

## Documentation
- Move the "Portability & Common Formats" overview from `src/value.rs` into a
  dedicated chapter of the book.
- Migrate the blob module introduction in `src/blob.rs` so the crate docs focus
  on API details.
- Extract the repository design discussion and Git parallels from `src/repo.rs`
  into the book.
- Split out the lengthy explanation of trible structure from `src/trible.rs`
  and consolidate it with the deep dive chapter.

## Discovered Issues
- No open issues recorded yet.

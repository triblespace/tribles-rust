# Inventory

## Potential Removals
- None at the moment.

## Desired Functionality
- Provide additional examples showcasing advanced queries and repository usage.
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
- Implement a garbage collection mechanism that scans branch and commit
  archives without fully deserialising them to find reachable blob handles.
  Anything not discovered this way can be forgotten by the underlying store.

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

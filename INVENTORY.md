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

## Discovered Issues
- No open issues recorded yet.

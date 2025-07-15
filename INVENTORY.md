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
  that receives the previous and current `TribleSet` and matches only
  the newly added tribles. See the book's
  [Incremental Queries](book/src/incremental-queries.md) chapter for
  the planned approach.

## Discovered Issues
- No open issues recorded yet.

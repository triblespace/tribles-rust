# Incremental Queries

The query engine today evaluates against a static `TribleSet`. Many
applications would rather compute just the new results when additional
data arrives. We support this using *semi‑naive evaluation*.

When a dataset grows, we compute a delta set of the newly inserted
tribles. For each triple constraint in the original query we evaluate a
variant where only that constraint is restricted to the delta while the
remaining constraints see the full updated dataset. Each case yields the
new solutions introduced by those additions and we union all of the
per‑constraint results.

## Monotonicity and CALM

Removed results are not tracked. Tribles follow the [CALM
principle](https://bloom-lang.net/calm/): a program whose outputs are
monotonic in its inputs needs no coordination. Updates simply add new
facts and previously derived conclusions remain valid. When conflicting
information arises, applications append fresh tribles describing their
preferred view instead of retracting old ones. Stores may forget obsolete
data, but semantically tribles are never deleted.

## Example

Namespaces provide a `pattern_changes!` operator to express these delta
queries. It behaves like `pattern!` but takes the current `TribleSet` and
a precomputed changeset. The macro unions variants of the query where
each triple is constrained to that changeset, matching only the newly
inserted tribles. Combined with the union constraint, this lets us run
incremental updates using the familiar `find!` interface.

```rust
{{#include ../../examples/pattern_changes.rs:pattern_changes_example}}
```

## Comparing history points

`Workspace::checkout` accepts [commit selectors](commit-selectors.md)
which can describe ranges in repository history. By checking out a
range like `a..b` we obtain exactly the tribles introduced by commits
reachable from `b` but not from `a`. Feeding that delta into
`pattern_changes!` lets us ask, “What new matches did commit `b`
introduce over `a`?”

## Trade‑offs

- Applications must compute and supply the delta set; the engine does not
  track changes automatically.
- Queries must remain monotonic since deletions are ignored.
- Each triple incurs an extra variant, so highly selective constraints
  keep incremental evaluation efficient.

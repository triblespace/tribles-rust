# Incremental Queries

The query engine normally evaluates a pattern against a complete
`TribleSet`, recomputing every match from scratch. Applications that
ingest data continuously often only need to know which results are
introduced by new tribles. Tribles supports this with *semi‑naive
evaluation*, a classic incremental query technique.

## Delta evaluation

Given a base dataset and a set of newly inserted tribles, the engine runs
the original query multiple times. Each run restricts a different triple
constraint to the delta while the remaining constraints see the full set.
The union of these runs yields exactly the new solutions. The process is:

1. compute a `delta` `TribleSet` containing only the inserted tribles,
2. for every triple in the query, evaluate a variant where that triple
   matches against `delta`,
3. union all per‑triple results to obtain the incremental answers.

Because each variant touches only one triple from the delta, the work
grows with the number of constraints and the size of the delta set
rather than the size of the full dataset.

## Monotonicity and CALM

Removed results are not tracked. Tribles follow the
[CALM principle](https://bloom-lang.net/calm/): a program whose outputs
are monotonic in its inputs needs no coordination. Updates simply add new
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
which can describe ranges in repository history. By checking out a range
like `a..b` we obtain exactly the tribles introduced by commits reachable
from `b` but not from `a`. Feeding that delta into `pattern_changes!`
lets us ask, “What new matches did commit `b` introduce over `a`?”

## Trade‑offs

- Applications must compute and supply the delta set; the engine does not
  track changes automatically.
- Queries must remain monotonic since deletions are ignored.
- Each triple incurs an extra variant, so highly selective constraints
  keep incremental evaluation efficient.

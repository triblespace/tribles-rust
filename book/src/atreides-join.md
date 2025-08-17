# The Atreides Family of Worst-case Optimal Join Algorithms

The heart of the system is a constraint-solving approach based on the theory of worst-case optimal joins. Queries are represented as sets of constraints over variables, and the join algorithm explores this search space by binding one variable at a time.

Size estimates, normally used only by query optimizers, directly guide the join. Instead of building explicit join plans, the engine asks each constraint for cardinality bounds and chooses the next variable with the smallest estimated result. Traditional databases rely on sorted indexes to efficiently find corresponding values. Atreides still uses random lookups when confirming each candidate, but the bounds let it enumerate the most specific constraint sequentially and probe only a few possibilities in the larger ones, offering similar pruning power without maintaining sorted indexes everywhere.

Maintaining accurate estimates is crucial. We therefore provide data structures that update cardinalities in **O(1)** time so the algorithm can adapt quickly as bindings accumulate.

We currently expose four variants of the Atreides family, each with a descriptive name and a playful Dune nickname. They differ in how much information they consider when predicting the remaining search space:

- **Row-count Join (Jessica)** – uses per-constraint row counts to pick the variable with the smallest number of matching rows.
- **Distinct-value Join (Paul)** – estimates the smallest number of distinct values for the variable across one column.
- **Partial-binding Join (Ghanima)** – refines the estimate using the current partial binding but ignores yet-to-be-bound variables.
- **Exact-result Join (Leto)** – the ideal algorithm that knows the exact result size even for unbound variables.

Each variant trades complexity for precision. More accurate estimates let the engine prune failing paths earlier.

For example, given constraints that relate `?person` to `?parent` and `?city`, the Distinct-value Join (Paul) will bind whichever variable has the fewest distinct candidates. If `?city` only has a handful of possibilities, the search tries them first and backtracks quickly when they fail.

The join proceeds as a depth-first search. At every step the engine binds a value, evaluates the affected constraints, and backtracks when a constraint reports no matches. Because estimates come directly from the constraints, the engine avoids complex query planning and remains resistant to data skew — even wildly imbalanced cardinalities do not degrade performance.

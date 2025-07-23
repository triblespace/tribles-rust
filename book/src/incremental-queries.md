# Incremental Queries

The query engine today evaluates against a static `TribleSet`. Many
applications would rather compute just the new results when additional
data arrives. We plan to support this using *semi‑naive evaluation*.

When a dataset grows, we compute a delta set of the newly inserted
tribles. For each triple constraint in the original query we evaluate a
variant of the query where that single constraint is restricted to the
delta while the remaining constraints see the full updated dataset. Each
case yields the new solutions introduced by those additions and we then
union all of the per‑constraint results.

To help express these delta queries at the macro level, namespaces now
offer a `delta!` operator. It behaves like `pattern!` but takes the
previous and current `TribleSet`. The macro computes their difference
and then calls `union!` internally to apply the resulting delta
constraint, matching only the newly inserted tribles. Combined with the
union constraint, this lets us run incremental updates using the familiar
`find!` interface.

We can reuse the existing `find!` interface to run these partial queries
and poll for updates whenever an application receives a new `TribleSet`.
This mechanism also lets us compute the difference between two arbitrary
datasets by treating the change set as the delta.

Removed results are not tracked. Tribles are designed to be monotonic
(CALM); new facts never invalidate previous conclusions. Applications can
represent contradictions explicitly and continue operating by appending
new tribles that reference the view they choose to follow.

Semantically this means tribles are never deleted, though individual
stores may forget them. Old data remains valid even if it becomes
inaccessible.

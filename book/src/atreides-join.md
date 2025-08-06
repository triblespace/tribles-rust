# The Atreides Family of Worst-case Optimal Join Algorithms

The heart of the system is a constraint-solving approach based on the theory
of worst-case optimal joins, specifically a family of novel join algorithms
we call the "Atreides Family".

The key insight is that size estimations, normally used by query optimizers,
can directly guide the join algorithm to retrieve bounds that typically require
sorted indexes for random access.

This shifts much of the execution cost to cardinality estimation, so we developed
novel data structures to efficiently maintain these estimates in O(1) time.

We focus on three specific instantiations of the "Atreides Family",
which differ in the quality of the cardinality estimation provided, i.e.,
the clarity the algorithm has when looking into the future.

Given a _partial_ Binding:

- *Jessica's Join* - Estimates the smallest number of rows matching the variable.
- *Paul's Join* - Estimates the smallest number of distinct values from one column matching the variable.
- *Ghanima's Join* - Estimates the number of values matching the variable with the given binding, without considering yet-to-be-bound variables.
- *Leto's Join* - Estimates the true number of values matching the variable with the given binding, considering all variables, even those not yet bound.

The algorithm uses a depth-first search, where the query engine tries to find
a solution by iteratively proposing values for the variables and backtracking when it reaches a dead end.
The constraints are not evaluated in a fixed order; instead, the query engine uses the
estimates provided by the constraints to guide the search.
This allows for a more flexible and efficient exploration of the search space,
as the query engine can focus on the most promising parts.
This also obviates the need for complex query optimization techniques, as the
constraints themselves provide the necessary information to guide the search,
and the query engine can adapt dynamically to the data and the query, providing
skew-resistance and predictable performance. Meaning that the query engine can
handle queries that have a wide range of variances in the cardinalities of the variables,
without suffering from performance degradation.

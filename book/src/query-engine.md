# Query Engine

Queries retrieve data by describing the patterns you are looking for. The engine
pursues extreme simplicity, low and predictable latency, skew resistance and
requires no tuning. New constraints implement the
[`Constraint`](crate::query::Constraint) trait so different operators,
sub-languages and data sources can be composed.

## Queries as Schemas

You might have noticed that trible.space does not have a concept of an ontology
or schema specification beyond associating attributes with
[`ValueSchema`](crate::value::ValueSchema) and
[`BlobSchema`](crate::prelude::BlobSchema). This is deliberate. One of our
lessons from the semantic web was that it is too loose in typing individual
values, but too strict and computationally infeasible in describing larger
structures. Any system dealing with real-world data must handle cases of
missing, duplicate or additional fields, which conflicts with strong
constraints like classes.

Our approach is to be sympathetic to edge cases and have the system deal only
with the data it declares capable of handling. These "application-specific
schema declarations" are exactly the shapes and constraints described by our
queries[^1]. Data not conforming to these queries/schemas is simply ignored by
definition (as a query only returns data conforming to its constraints).[^2]

## Join Strategy

The query engine uses the Atreides family of worst-case optimal join
algorithms. These algorithms leverage cardinality estimates to guide a
depth-first search over variable bindings, providing skew-resistant and
predictable performance. For a detailed discussion, see the [Atreides
Join](atreides-join.md) chapter.

## Query Languages

There is no query language in the traditional sense, but rather a set of
constraints that can be combined using logical operators like `and` and `or`.
The constraints are designed to be simple and flexible, allowing for a wide
range of constraints to be implemented while still permitting efficient
exploration of the search space by the query engine.

The query engine and data model is flexible enough to allow for the exploration
of a wide range of query languages, including graph queries, relational queries
and document queries.

For example the [`namespace`](crate::namespace) module provides a set of macros
that allow for the easy creation of constraints for a given trible pattern, with
a syntax similar to query-by-example languages like SPARQL or GraphQL,
tailored to a document-graph oriented data model. But it would also be possible
to implement a property graph query language like Cypher, or a relational query
language like Datalog, on top of the query engine.[^3]

Great care has been taken to ensure that query languages with different styles
and semantics can be easily implemented on top of the query engine, while
allowing them to be mixed and matched with other languages and data models in
the same query. For practical examples of the current facilities, see the
[Query Language](query-language.md) chapter.

[^1]: Note that this query-schema isomorphism isn't necessarily true in all
databases or query languages, e.g., it does not hold for SQL.
[^2]: In RDF terminology: We challenge the classical A-Box & T-Box dichotomy by
replacing the T-Box with a "Q-Box", which is descriptive and open rather than
prescriptive and closed. This Q-Box naturally evolves with new and changing
requirements, contexts and applications.
[^3]: SQL would be a bit more challenging, as it is surprisingly imperative
with its explicit JOINs and ORDER BYs, and its lack of a clear declarative
semantics. This makes it harder to implement on top of a constraint-based query
engine tailored towards a more declarative and functional style.

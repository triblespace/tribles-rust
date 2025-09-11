# Query Language

This chapter introduces the basic query facilities provided by `tribles`.
Queries are declared in a small declarative language that describes which
values should match rather than how to iterate them.

The [`find!`](crate::prelude::find) macro builds a
[`Query`](crate::query::Query) by declaring variables and a constraint
expression. A minimal invocation looks like:

```rust
let results = find!((a), a.is(1.into())).collect::<Vec<_>>();
```

`find!` returns an [`Iterator`](core::iter::Iterator) over tuples of the bound
variables. Matches can be consumed lazily or collected into common
collections:

```rust
for (a,) in find!((a), a.is(1.into())) {
    println!("match: {a}");
}
```

Variables can optionally specify a concrete type to convert the underlying
value:

```rust
find!((x: i32, y: Value<ShortString>),
      and!(x.is(1.into()), y.is("foo".to_value())))
```

The first variable is read as an `i32` and the second as a short string if the
conversion succeeds. The query engine walks all possible assignments that
satisfy the constraint and yields tuples of the declared variables.

## Built-in constraints

`find!` queries combine a small set of constraint operators to form a declarative
language for matching tribles:

- [`and!`](crate::prelude::and) builds an
  [`IntersectionConstraint`](crate::query::intersectionconstraint::IntersectionConstraint)
  requiring all sub-constraints to hold.
- [`or!`](crate::prelude::or) constructs a
  [`UnionConstraint`](crate::query::unionconstraint::UnionConstraint)
  accepting any satisfied alternative.
- [`ignore!`](crate::ignore) tells the query engine to ignore variables in
  a sub-query. Constraints mentioning ignored variables are evaluated without
  checking those positions, so their bindings neither join with surrounding
  constraints nor appear in the result set.
- Collection types such as [`TribleSet`](crate::tribleset::TribleSet) provide a
  `has` method yielding a
  [`ContainsConstraint`](crate::query::hashsetconstraint::ContainsConstraint) for
  membership tests.

Any data structure that can iterate its contents, test membership and report its
size can implement `ContainsConstraint`, so queries have no inherent ordering
requirements.

Ignored variables are handy when a sub-expression references fields you want to
drop. The engine skips checking them entirely, effectively treating the
positions as wildcards. If the underlying constraint guarantees some value,
ignoring works like existential quantification; otherwise the ignored portion is
simply discarded. Without ignoring, those variables would leak into the outer
scope and either appear in the results or unintentionally join with other
constraints.

Alternatives are expressed with `or!` and temporary variables can be hidden
with `ignore!`:

```rust
find!((x), or!(x.is(1.into()), x.is(2.into())));

find!((x),
      ignore!((y), and!(x.is(1.into()), y.is(2.into()))));
```

In the second query `y` is ignored entirely—the engine never checks the
`y.is(2.into())` part—so the outer query only enforces `x.is(1.into())`
regardless of whether any `y` equals `2`.

## Example

```rust
use tribles::prelude::*;
use tribles::examples::{self, literature};

let dataset = examples::dataset();

for (title,) in find!((title: Value<_>),
                     and!(dataset.has(title), title.is("Dune".to_value()))) {
    println!("Found {}", title.from_value::<&str>());
}
```

This query searches the example dataset for the book titled "Dune".  The
variables and constraint can be adapted to express more complex joins and
filters.

## `matches!`

Sometimes you only want to check whether a constraint has any solutions.
The `matches!` macro mirrors the `find!` syntax but returns a boolean:

```rust
use tribles::prelude::*;

assert!(matches!((x), x.is(1.into())));
assert!(!matches!((x), and!(x.is(1.into()), x.is(2.into()))));
```

## Custom constraints

Every building block implements the
[`Constraint`](crate::query::Constraint) trait.  You can implement this trait on
your own types to integrate custom data sources or query operators with the
solver.

## Regular path queries

The `path!` macro lets you search for graph paths matching a regular
expression over edge attributes.  It expands to a
[`RegularPathConstraint`](crate::query::RegularPathConstraint) and can be
combined with other constraints.  Invoke it through a namespace module
(`social::path!`) to implicitly resolve attribute names:

```rust
use tribles::prelude::*;

mod social {
  use tribles::prelude::*;
  use tribles::prelude::valueschemas::*;
  attributes! {
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as follows: GenId;
    "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB" as likes: GenId;
  }
}
let mut kb = TribleSet::new();
let a = fucid(); let b = fucid(); let c = fucid();
kb += crate::entity!(&a, { social::follows: &b });
kb += crate::entity!( &b, { social::likes: &c });

let results: Vec<_> = find!((s: Value<_>, e: Value<_>),
    path!(&kb, s (social::follows | social::likes)+ e)).collect();
```

The middle section uses a familiar regex syntax to describe allowed edge
sequences.  Editors with Rust macro expansion support provide highlighting and
validation of the regular expression at compile time. Paths reference
attributes from a single namespace; to traverse edges across multiple
namespaces, create a new namespace that re-exports the desired attributes and
invoke `path!` through it.

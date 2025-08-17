# Query Language

This chapter introduces the basic query facilities provided by `tribles`.

The [`find!`](crate::prelude::find) macro builds a [`Query`](crate::query::Query)
by declaring variables and a constraint expression.  A minimal invocation looks
like:

```rust
let results = find!((a), a.is(1.into())).collect::<Vec<_>>();
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
- [`mask!`](crate::prelude::mask) hides variables from a sub-query.
- Collection types such as [`TribleSet`](crate::tribleset::TribleSet) provide a
  `has` method yielding a
  [`ContainsConstraint`](crate::query::hashsetconstraint::ContainsConstraint) for
  membership tests.

Any data structure that can iterate its contents, test membership and report its
size can implement `ContainsConstraint`, so queries have no inherent ordering
requirements.

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
NS! { namespace social {
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" as follows: GenId;
    "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB" as likes: GenId;
} }
let mut kb = TribleSet::new();
let a = fucid(); let b = fucid(); let c = fucid();
kb += social::entity!(&a, { follows: &b });
kb += social::entity!(&b, { likes: &c });

let results: Vec<_> = find!((s: Value<_>, e: Value<_>),
    social::path!(&kb, s (follows | likes)+ e)).collect();
```

The middle section uses a familiar regex syntax to describe allowed edge
sequences.  Editors with Rust macro expansion support provide highlighting and
validation of the regular expression at compile time. Paths reference
attributes from a single namespace; to traverse edges across multiple
namespaces, create a new namespace that re-exports the desired attributes and
invoke `path!` through it.

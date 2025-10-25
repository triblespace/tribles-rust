# Descriptive Structural Typing and the find! Idiom

This chapter documents the mental model and idioms we recommend when working
with tribles. The model is intentionally descriptive: queries declare the
shape of the data you want to see rather than prescribing a single concrete
Rust type for every entity. This gives you the flexibility to keep the full
graph around and to materialize only the view you need, when you need it.

## Key ideas at a glance

- Attributes are typed fields (unlike untyped RDF predicates).
- An entity in tribles is a structural record: a bag of typed fields.
- find! patterns are descriptive type checks / projections: they select
  entities that match a requested shape.
- entity! constructs ad‑hoc entities (like struct literals).
- Reified kinds/tags are attached via metadata::tag (GenId); projects often
  export canonical KIND_* constants you can pattern-match directly against.
- Prefer passing the Workspace + the checkout result (TribleSet) and an
  entity id around — only materialize a concrete Rust view when required.

## Why "descriptive" not "prescriptive"?

In a prescriptive system you define a named struct (type), commit to it, and
force conversions at boundaries. Tribles instead let you describe the fields
you need at call sites. That keeps code resilient to schema evolution and
avoids unnecessary unfolding of the graph.

In linguistic terms: instead of insisting every entity be declared as
CategoryX, you ask "show me entities that have fields A and B" and work with
those. If an entity also has field C that's fine — it simply matches the
descriptive pattern.

## Type theory mapping (short)

- Structural typing: types are shapes of fields, not names.
- Width subtyping: records with more fields subsume records with fewer.
- Intersection types: requiring both patterns A and B is like A & B.
- Row polymorphism: patterns naturally allow additional (unspecified)
  fields to exist.

## Core idioms and recommended patterns

### 1. Use Workspace as your core I/O handle

The Workspace is the primary object for interacting with a repository. It
lets you open a branch, commit, push, checkout history, and — importantly —
read blob handles (LongString) cheaply.

Pattern: open a workspace for the configured branch, checkout the HEAD
ancestors to produce a TribleSet content snapshot for efficient read-only
pattern matching, and use the same Workspace to lazily read blobs when you
need them.

This avoids duplicating memory and allows cheap zero-copy access to LongString
blobs.

#### Manager-owned repository and workspace DI

At runtime, prefer to give a long-lived manager (session, exporter, service)
ownership of a Repository instance and expose an "open_workspace()" helper
that returns a Workspace for branch-local reads/writes. Library functions
should accept a &mut Workspace (or a TribleSet + Workspace) as arguments
instead of opening piles/repositories themselves. This avoids repeated
Pile::open + Repository::new churn in hot paths and centralizes lifecycle
management in one place.

Example (pseudocode):

```rust
// manager owns a Repository for the process/session lifetime
let mut repo = manager.repo.open_workspace()?;
let mem = memory_for_prompt_ws(&mut repo, ...)?; // workspace-variant helper
```

This pattern keeps startup/teardown centralized and eliminates the common
hot-loop anti-pattern of repeatedly opening ephemeral repository instances.

### 2. Use find! as a descriptive type / projection

find! is not just a query language; it is the place where you declare the
shape of the data you expect. Treat find! patterns as lightweight, inline
type declarations. If an entity doesn't match, find! won't return it — no
error, just absence.

When your project defines canonical tag ids (GenId constants) prefer to
match the tag directly in the pattern rather than binding a short-string
and filtering afterwards.

Example: find plan snapshot ids (match tag directly)

```rust
// Match entities that have the canonical plan snapshot tag attached.
for (e,) in find!((e: Id), triblespace::pattern!(&content, [{ ?e @ metadata::tag: (KIND_PLAN_SNAPSHOT) }])) {
    // `e` is a plan snapshot entity id; follow-up finds can read other fields
}
```

### 3. Lazy, ad‑hoc conversions only where needed

If a function needs a few fields for an operation, ask for them with find!
inside the function. If later you perform an operation that needs different
fields, you can perform another small find! there. Don't materialize large
subgraphs unless a single operation needs them.

The recommended function signature is minimal and focused on the
tribles primitives:

```rust
fn handle_plan_update(ws: &mut Workspace<Pile<Blake3>>, plan_id: Id) -> io::Result<()> {
    // ad-hoc find! calls to read the fields we need
}
```

### 4. Read LongString as &str (zero-copy)

Blob schema types in tribles are intentionally zerocopy. Prefer the
typed View API which returns a borrowed &str without copying when possible.

```rust
let view = ws.get::<View<str>, LongString>(handle).map_err(|e| ...)?;
let s: &str = view.as_ref(); // zero-copy view tied to the workspace lifetime
// If you need an owned String: let owned = view.to_string();
```

Note: a View borrows data that is managed by the Workspace; avoid returning
`&str` that outlives the workspace or the View.

### 5. Structural sharing and normalization patterns

When persisting graphs that contain many repeated or immutable pieces
(e.g. steps in a plan), prefer structural sharing:
- Store canonical step entities (LongString blobs for their text).
- Create a lightweight "link" entity per plan that references the step ids
  and metadata like order and status.

On update, create new step entities only for truly new step text and add a
new snapshot entity that references the steps. This keeps history immutable
and easy to reason about.

### 6. Push/merge retry loop for writers

When pushing writes, use the standard push/merge loop to handle concurrent
writers. Two options are available:

- Manual conflict handling with `try_push` (single attempt; returns a
  conflicting workspace on CAS failure):

```rust
ws.commit(content, Some("plan-update"));
let mut current_ws = ws;
while let Some(mut incoming) = repo.try_push(&mut current_ws)? {
    incoming.merge(&mut current_ws)?;
    current_ws = incoming;
}
```

- Automatic retries with `push` (convenience wrapper that merges and retries
  until success or error):

```rust
ws.commit(content, Some("plan-update"));
// `push` will handle merge+retry internally; it returns Ok(None) on success
// or an error if the operation ultimately failed.
repo.push(&mut ws)?;
```

## Practical anti‑patterns
- Do not unfold the graph or convert it into nested Rust structs.
  It wastes CPU and memory and loses the benefits of tribles’ flexible
  reifications.
- Avoid holding repo locks across async/await points. Acquire workspaces,
  do the minimal synchronous I/O you need, then release locks before awaiting.
- Don’t assume presence of a field; be explicit about optional vs required
  semantics using Option / Result in typed adapters.
- Don't create ephemeral Repository instances every time you need to read or
  write data. Instead, own a long-lived Repository in a manager and expose
  workspace-opening helpers.
- Don't explicitly convert Values via from_value/to_value; use typed `find!`
  patterns `find!((field: <field_type>), ...)` and
  `ws.get::<View<...>, _>(handle)` for blob reads.
- Don't create helper functions for queries, use `find!` patterns directly in
  the function that needs them. This keeps the shape of the data explicit and
  close to where it is used and avoids unnecessary unfolding of the graph.
- Don't convert the structures returned by `find!` into other Rust structs.
  Work with the returned tuples directly.
- `find!` should be thought of like a fundamental language primitive, like
  `if`/`match`/`for`. It is not a "database query" that returns rows to be
  converted into structs, it is a way to describe the shape of the data
  you want to see. ORM-like abstractions that try to map `find!` results
  into structs are an anti-pattern.
- Avoid reading blobs eagerly; prefer lazy reads via `ws.get::<View<...>, _>(handle)`.
  Allocate owned data only when necessary.

## Glossary

- Workspace: the repo handle that opens branches, reads blobs, commits and
  pushes.
- TribleSet: the in-memory content snapshot returned by Workspace::checkout.
- find!: the macro you use to discover entities matching a pattern (a
  descriptive type declaration).
- entity!: construct an ad‑hoc entity into a TribleSet for commit.
- LongString: zero-copy blob schema for potentially-large text.

## Closing notes

This chapter captures the pragmatic type story we use in tribles: describe
the fields you need at the place you need them, keep the full graph, and
materialize small views lazily.

## Reviewers' checklist (quick)

- Prefer find!/pattern! projections for the fields needed by the function.
- Avoid converting graph into rust structs.
- Use ws.get::<View<...>, _>(handle) for zero-copy blob reads;
  allocate only when an owned value is required.
- Match canonical tag ids via metadata::tag (KIND_* constants).
- Manager-owned repo: long-lived Repository instances should be owned by a
  session/exporter/manager; library code should accept a Workspace or
  TribleSet rather than opening piles itself.
- Use push/merge retry loops for writers; avoid holding repo locks across
  async/await points.

The sections below contain copy‑pasteable recipes for common operations.

## Idioms & code recipes

This section contains pragmatic, copy‑pasteable snippets and patterns you can
reuse. The examples intentionally use the tribles macros (attributes!, find!,
pattern!, entity!) directly — that is the intended style.

### Reviewer checklist

When reviewing code that touches tribles, look for these items:

- Does the code use find! to select only the fields it needs, rather than
  unfolding the entire graph?
- Are blob reads kept lazy (only read LongString when necessary)?
- Are push flows using the push/merge retry loop to avoid losing concurrent
  updates?
- Is the code avoiding holding the repo's Mutex across awaits and long
  blocking operations?

## Further reading and references

- See the tribles macros: attributes!, find!, pattern!, entity! in the tribles code
  for exact usage.
- Type theory: "row polymorphism", "structural typing", "width subtyping"
  if you want the formal background.

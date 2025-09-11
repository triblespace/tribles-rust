---
title: Descriptive Structural Typing and the find! Idiom
weight: 30
---

This chapter documents the mental model and idioms we recommend when working
with tribles. The model is intentionally descriptive: queries declare the
shape of the data you want to see rather than prescribing a single concrete
Rust type for every entity. This gives you the flexibility to keep the full
graph around and to materialize only the view you need, when you need it.

Key ideas at a glance
- Namespace attributes are typed fields (not untyped RDF predicates).
- An entity in tribles is a structural record: a bag of typed fields.
- find! patterns are descriptive type checks / projections: they select
  entities that match a requested shape.
- entity! constructs ad‑hoc entities (like struct literals).
- Reified kinds/tags are attached via metadata::tag (GenId); projects often
  export canonical KIND_* constants you can pattern-match directly against.
- Prefer passing the Workspace + the checkout result (TribleSet) and an
  entity id around — only materialize a concrete Rust view when required.

Why "descriptive" not "prescriptive"?
-------------------------------------

In a prescriptive system you define a named struct (type), commit to it, and
force conversions at boundaries. Tribles instead lets you describe the fields
you need at call sites. That keeps code resilient to schema evolution and
avoids unnecessary unfolding of the graph.

In linguistic terms: instead of insisting every entity be declared as
CategoryX, you ask "show me entities that have fields A and B" and work with
those. If an entity also has field C that's fine — it simply matches the
descriptive pattern.

Type theory mapping (short)
- Structural typing: types are shapes of fields, not names.
- Width subtyping: records with more fields subsume records with fewer.
- Intersection types: requiring both patterns A and B is like A & B.
- Row polymorphism: patterns naturally allow additional (unspecified)
  fields to exist.

Core idioms and recommended patterns
-----------------------------------

1) Use Workspace as your core I/O handle
---------------------------------------

The Workspace is the primary object for interacting with a repository. It
lets you open a branch, commit, push, checkout history, and — importantly —
read blob handles (LongString) cheaply.

Pattern: open a workspace for the configured branch, checkout the HEAD
ancestors to produce a TribleSet content snapshot for efficient read-only
pattern matching, and use the same Workspace to lazily read blobs when you
need them.

This avoids duplicating memory and allows cheap zero-copy access to LongString
blobs.

Manager-owned repository and workspace DI
----------------------------------------

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

2) Use find! as a descriptive type / projection
----------------------------------------------

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
for (e,) in find!((e: Id), tribles::pattern!(&content, [{ e @ metadata::tag: (KIND_PLAN_SNAPSHOT) }])) {
    // `e` is a plan snapshot entity id; follow-up finds can read other fields
}
```

3) Lazy, ad‑hoc conversions only where needed
-------------------------------------------

If a function needs a few fields for an operation, ask for them with find!
inside the function. If later you perform an operation that needs different
fields, you can perform another small find! there. Don't materialize large
subgraphs unless a single operation needs them.

The recommended function signature is minimal and focused on the
tribles primitives:

```rust
fn handle_plan_update(ws: &mut Workspace<Pile<Blake3>>, content: &TribleSet, plan_id: Id) -> io::Result<()> {
    // ad-hoc find! calls to read the fields we need
}
```

4) Read LongString as &str (zero-copy)
-------------------------------------

Blob schema types in tribles are intentionally zerocopy. Prefer the
typed View API which returns a borrowed &str without copying when possible.

```rust
let view = ws.get::<View<str>, LongString>(handle).map_err(|e| ...)?;
let s: &str = view.as_ref(); // zero-copy view tied to the workspace lifetime
// If you need an owned String: let owned = view.to_string();
```

Note: a View borrows data that is managed by the Workspace; avoid returning
`&str` that outlives the workspace or the View.

5) Structural sharing and normalization patterns
-----------------------------------------------

When persisting graphs that contain many repeated or immutable pieces
(e.g. steps in a plan), prefer structural sharing:
- Store canonical step entities (LongString blobs for their text).
- Create a lightweight "link" entity per plan that references the step ids
  and metadata like order and status.

On update, create new step entities only for truly new step text and add a
new snapshot entity that references the steps. This keeps history immutable
and easy to reason about.

6) Push/merge retry loop for writers
------------------------------------

When pushing writes, use the standard push/merge loop to handle concurrent
writers:

```rust
ws.commit(content, Some("plan-update"));
let mut current_ws = ws;
while let Some(mut incoming) = match repo.push(&mut current_ws) {
    Ok(Some(i)) => Ok(Some(i)),
    Ok(None) => Ok(None),
    Err(e) => Err(io::Error::new(...)),
}? {
    incoming.merge(&mut current_ws)?;
    current_ws = incoming;
}
```

Worked example: reading a plan snapshot with lazy step texts
-----------------------------------------------------------

This example demonstrates the recommended pattern: checkout content, use
find! to project the fields you need, and read step texts lazily via the
Workspace when required.

```rust
use tribles::blob::schemas::longstring::LongString;
use tribles::value::schemas::shortstring::ShortString;

fn get_plan_with_steps(ws: &mut Workspace<Pile<Blake3>>, plan_hex: &str) -> io::Result<Option<PlanDetail>> {
    let plan_id = parse_hex_to_id(plan_hex).ok_or_else(|| ...)?;

    // checkout a stable read-only view of the history
    let head = ws.head().ok_or_else(|| io::Error::new(...))?;
    let content = ws.checkout(tribles::repo::ancestors(head))?;

    // find the plan snapshot entity id and its explanation
    let mut explanation = None;
    for (e, h) in find!((e: Id, h: Value<Handle<Blake3, LongString>>), crate::pattern!(&content, [{ e @ planner::explanation: h }])) {
        if e == plan_id {
            explanation = Some(read_longstring_as_str(ws, h)?);
            break;
        }
    }

    // collect linked step entities (the links are lightweight)
    let mut steps: Vec<PlanStepRef> = Vec::new();
    // Match entities that have the canonical "plan_step" tag. Projects
    // should export a KIND_PLAN_STEP constant for this purpose.
    for (link,) in find!((link: Id), tribles::pattern!(&content, [{ link @ metadata::tag: (KIND_PLAN_STEP) }])) {
        // ensure it references our plan_id and then extract step_id,
        // step_index, step_status and store the step_text handle in PlanStepRef
        // without reading the blob yet (lazy).
    }

    // later: lazy read
    for mut step in steps.iter_mut() {
        if let Some(h) = step.text_handle.take() {
            let s = read_longstring_as_str(ws, h)?; // zero-copy conversion
            step.text = Some(s.to_string()); // or keep as &str if lifetime allows
        }
    }

    Ok(Some(PlanDetail { explanation, steps }))
}

fn read_longstring_as_str(ws: &mut Workspace<Pile<Blake3>>, h: Value<Handle<Blake3, LongString>>) -> io::Result<&str> {
    let blob: tribles::blob::Blob<LongString> = ws.get::<_, LongString>(h).map_err(|e| io::Error::new(...))?;
    std::str::from_utf8(blob.bytes.as_ref()).map_err(|_| io::Error::new(...))
}
```

Practical anti‑patterns
------------------------
- Do not eagerly unfold the entire graph into a giant nested Rust struct.
  It wastes CPU and memory and loses the benefits of tribles’ flexible
  reifications.
- Avoid holding repo locks across async/await points. Acquire workspaces,
  do the minimal synchronous I/O you need, then release locks before awaiting.
- Don’t assume presence of a field; be explicit about optional vs required
  semantics using Option / Result in typed adapters.

Testing and validation
----------------------

Because find! is descriptive, missing matches may be a silent symptom of a
bug or of schema evolution. Use targeted validators in critical code paths:
- tests that use TryFrom adapters and assert expected fields are present,
- property tests that generate plan updates and assert round-trip persistence,
- small runtime assertions in critical workflows that fail loudly if a
  required field is missing.

Glossary
--------
- Workspace: the repo handle that opens branches, reads blobs, commits and
  pushes.
- TribleSet: the in-memory content snapshot returned by Workspace::checkout.
- find!: the macro you use to discover entities matching a pattern (a
  descriptive type declaration).
- entity!: construct an ad‑hoc entity into a TribleSet for commit.
- LongString: zero-copy blob schema for potentially-large text.

Closing notes
-------------

This chapter captures the pragmatic type story we use in tribles: describe
the fields you need at the place you need them, keep the full graph, and
materialize small views lazily.

Reviewers' checklist (quick)
- Prefer find!/pattern! projections for the fields needed by the function.
- Avoid unfolding the entire graph into a giant nested struct for simple reads.
- Use ws.get::<View<str>, LongString>(handle) for zero-copy blob reads;
  allocate only when an owned String is required.
-- Match canonical tag ids via metadata::tag (KIND_* constants).
- Manager-owned repo: long-lived Repository instances should be owned by a
  session/exporter/manager; library code should accept a Workspace or
  TribleSet rather than opening piles itself.
- Use push/merge retry loops for writers; avoid holding repo locks across
  async/await points.

The sections below contain copy‑pasteable recipes for common operations.

Idioms & code recipes
---------------------

This section contains pragmatic, copy‑pasteable snippets and patterns you can
reuse. The examples intentionally use the tribles macros (attributes!, find!,
pattern!, entity!) directly — that is the intended style.

EntityRef: a tiny ergonomic helper (optional)
-------------------------------------------

If you find yourself repeating the same find! patterns to read a few fields
from the same entity, a small helper type can reduce boilerplate. This is an
optional convenience — the idiomatic approach is still to use find! where
you need it.

```rust
/// Minimal ergonomic wrapper for reading fields of an entity in a content snapshot.
struct EntityRef<'c, 'ws> {
    content: &'c TribleSet,
    ws: &'ws mut Workspace<Pile<Blake3>>,
    id: Id,
}

impl<'c,'ws> EntityRef<'c,'ws> {
    fn get_short(&self, attr: impl FnOnce() -> /* attribute macro slot */) -> Option<String> {
        // concept: use find! with the chosen attribute and return the ShortString
        // (details depend on how you pass the attribute into the macro).
        None
    }

    fn get_text_blob(&mut self, handle: Value<Handle<Blake3, LongString>>) -> io::Result<&str> {
        // zero-copy read: obtain blob and convert to &str (small UTF-8 check)
        let blob: tribles::blob::Blob<LongString> = self.ws.get::<_, LongString>(handle)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("blob read: {e:?}")))?;
        std::str::from_utf8(blob.bytes.as_ref()).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf8"))
    }
}
```

Filled-in example for common planner attributes
------------------------------------------------

If you only need a handful of named getters for a particular namespace, an
explicit small set of accessors is practical and readable. Below is a
concrete sketch for the planner namespace used throughout this chapter. The
implementation uses find! patterns directly — this is the style we prefer.

```rust
impl<'c,'ws> EntityRef<'c,'ws> {
    fn kind(&self) -> Option<String> {
        // Return the canonical tag id (hex) attached to this entity, if any.
        for (e, tagv) in find!((e: Id, tagv: Id), tribles::pattern!(&self.content, [{ e @ metadata::tag: tagv }])) {
            if e == self.id {
                return Some(format!("{:X}", tagv));
            }
        }
        None
    }

    fn explanation(&mut self) -> Option<String> {
        for (e, h) in find!((e: Id, h: Value<Handle<Blake3, LongString>>), crate::pattern!(&self.content, [{ e @ planner::explanation: h }])) {
            if e == self.id {
                // zero-copy read and convert to owned string for convenience
                let blob: tribles::blob::Blob<LongString> = self.ws.get::<_, LongString>(h).ok()?;
                return Some(String::from_utf8_lossy(blob.bytes.as_ref()).to_string());
            }
        }
        None
    }
}
```

End-to-end persist-and-read worked example
-----------------------------------------

Below is a more complete example that demonstrates a common developer
workflow: persist a plan update (with structural sharing of steps) and then
read that snapshot back lazily.

Note: this example is intended for learning; production code should reuse
canonical exporter/utilities provided by your repository (they centralize
repository initialization, tagging conventions and push/merge logic).

```rust
// Persist a plan update. Returns the created/existing plan exclusive id.
fn persist_plan_update(
    repo: &mut tribles::repo::Repository<TriblePile<Blake3>>,
    ws: &mut Workspace<Pile<Blake3>>,
    args: &UpdatePlanArgs,
    session_uuid: uuid::Uuid,
) -> std::io::Result<tribles::id::ExclusiveId> {
    // Determine plan id: reuse provided plan_id or allocate a new one
    let plan_e = if let Some(pid) = &args.plan_id {
        parse_hex_to_id(pid).map(tribles::id::ExclusiveId::force).unwrap_or_else(|| ufoid())
    } else {
        ufoid()
    };

    // Prepare TribleSet content for commit
    let now = hifitime::Epoch::now().unwrap_or_else(|_| hifitime::Epoch::from_unix_seconds(0.0));
    let session_gen = tribles::id::Id::try_from(session_uuid).expect("session uuid->genid");

    let mut content = TribleSet::new();
    // attach a canonical tag for plan snapshot instead of storing a short-string kind
    content += crate::entity!(&plan_e, {
        planner::plan_id: &plan_e,
        planner::session: &session_gen,
        planner::created_at: (now, now),
        planner::updated_at: (now, now),
        metadata::tag: &KIND_PLAN_SNAPSHOT,
    });

    if let Some(call) = &args.call_id {
        content += crate::entity!(&plan_e, { planner::call_id: call.as_str() });
    }

    if let Some(expl) = &args.explanation {
        if !expl.trim().is_empty() {
            let blob = ws.put::<LongString, String>(expl.clone());
            content += crate::entity!(&plan_e, { planner::explanation: blob });
        }
    }

    // Structural sharing: try to find existing step entities with the same text.
    let mut existing_map: std::collections::HashMap<String, tribles::id::Id> = std::collections::HashMap::new();
    if let Some(head) = ws.head() {
        if let Ok(history_content) = ws.checkout(tribles::repo::ancestors(head)) {
            for (e, h) in find!((e: Id, h: Value<Handle<Blake3, LongString>>), crate::pattern!(&history_content, [{ e @ planner::step_text: h }])) {
                if let Ok(blob) = ws.get::<_, LongString>(h) {
                    let text = String::from_utf8_lossy(blob.bytes.as_ref()).to_string();
                    existing_map.entry(text).or_insert(e);
                }
            }
        }
    }

    for (idx, item) in args.plan.iter().enumerate() {
        let step_text = item.step.trim().to_string();
        let status = match item.status {
            StepStatus::Pending => "pending",
            StepStatus::InProgress => "in_progress",
            StepStatus::Completed => "completed",
        };

        if let Some(&existing_e) = existing_map.get(&step_text) {
            // create a lightweight link that references the existing step
            let link_e = ufoid();
            content += crate::entity!(&link_e, {
                planner::plan_id: &plan_e,
                planner::step_id: existing_e,
                planner::step_index: &idx.to_string(),
                planner::step_status: status,
                planner::updated_at: (now, now),
                metadata::tag: &KIND_PLAN_STEP,
            });
        } else {
            // create a new step entity and link
            let step_e = ufoid();
            let step_blob = ws.put::<LongString, String>(step_text.clone());
            content += crate::entity!(&step_e, {
                planner::step_id: &step_e,
                planner::step_text: step_blob,
                planner::created_at: (now, now),
                planner::updated_at: (now, now),
                metadata::tag: &KIND_STEP,
            });
            let link_e = ufoid();
            content += crate::entity!(&link_e, {
                planner::plan_id: &plan_e,
                planner::step_id: &step_e,
                planner::step_index: &idx.to_string(),
                planner::step_status: status,
                planner::updated_at: (now, now),
                metadata::tag: &KIND_PLAN_STEP,
            });
        }
    }

            ws.commit(content, Some("plan-update"));

    // Push with merge/retry
    let mut current_ws = ws.clone();
    while let Some(mut incoming) = match repo.push(&mut current_ws) {
        Ok(Some(i)) => Ok(Some(i)),
        Ok(None) => Ok(None),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, format!("push: {e:?}"))),
    }? {
        incoming.merge(&mut current_ws)?;
        current_ws = incoming;
    }

    Ok(plan_e)
}

// Later, to read the plan lazily, use the plan_steps_from_content helper above
```

Notes on the example
--------------------

- The example shows structural sharing: identical step text reuses an
  existing step entity. This is important for history compactness and to
  avoid duplicating large blobs.
- The push/merge loop resolves conflicts by merging the remote workspace
  state into the current workspace and retrying the push.
- The plan id (ExclusiveId) is returned to the caller so UIs can display a
  stable reference.


Lazy PlanStep iterator (pattern + lazy text)
--------------------------------------------

A common pattern is to list step metadata (id, index, status) without
immediately deserializing the step text blobs. Read the text only when the
consumer actually needs it.

```rust
struct PlanStepRef {
    step_id: Id,
    index: usize,
    status: Option<String>,
    text_handle: Option<Value<Handle<Blake3, LongString>>>,
    // text: Option<String> // fill lazily if you want an owned copy
}

fn plan_steps_from_content(content: &TribleSet, plan_id: Id) -> Vec<PlanStepRef> {
    let mut out = Vec::new();

    // find all link entities that have the plan_step tag
    for (link_id,) in find!((link_id: Id), tribles::pattern!(content, [{ link_id @ metadata::tag: (KIND_PLAN_STEP) }])) {
        // ensure it references our plan_id and then extract step_id,
        // step_index, step_status and collect the text handle for later lazy read.
    }

    out
}

// Later, when you need the text (with a mutable ws):
impl PlanStepRef {
    fn text<'ws>(&mut self, ws: &'ws mut Workspace<Pile<Blake3>>) -> io::Result<Option<String>> {
        if let Some(h) = self.text_handle.take() {
            let blob: tribles::blob::Blob<LongString> = ws.get::<_, LongString>(h)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("blob read: {e:?}")))?;
            // return an owned String here if you need owned data
            return Ok(Some(String::from_utf8_lossy(blob.bytes.as_ref()).to_string()));
        }
        Ok(None)
    }
}
```

Persisting a plan update (worked example)
-----------------------------------------

Below is a condensed, end‑to‑end example showing the key ideas: create
canonical step entities (LongString blobs), create a plan snapshot entity
that links the steps, commit, and push with the merge/retry loop.

```rust
// Given args: ws (workspace), args: UpdatePlanArgs, session_id

let plan_e = ufoid(); // exclusive id for the snapshot
let now = Epoch::now().unwrap_or_else(|_| Epoch::from_unix_seconds(0.0));

let mut content = TribleSet::new();

// base plan snapshot entity
content += crate::entity!(&plan_e, {
    planner::plan_id: &plan_e,
    planner::session: &session_genid,
    planner::created_at: (now, now),
    planner::updated_at: (now, now),
    metadata::tag: &KIND_PLAN_SNAPSHOT,
});

// store explanation if present
if let Some(expl) = &args.explanation {
    if !expl.trim().is_empty() {
        let blob = ws.put::<LongString, String>(expl.clone());
        content += crate::entity!(&plan_e, { planner::explanation: blob });
    }
}

// steps: either reuse existing step entities or create new ones
for (idx, item) in args.plan.iter().enumerate() {
    let step_text = item.step.trim().to_string();
    // reuse lookup omitted here: if we have a previous step with identical
    // text we can reuse its id. Otherwise create step entity and a plan_step
    // link that references it with step_index, step_status.
}

ws.commit(content, Some("plan-update"));

// push with merge/retry
let mut current_ws = ws;
while let Some(mut incoming) = match repo.push(&mut current_ws) {
    Ok(Some(i)) => Ok(Some(i)),
    Ok(None) => Ok(None),
    Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("push: {e:?}"))),
}? {
    incoming.merge(&mut current_ws)?;
    current_ws = incoming;
}

// return plan id to caller (hex)
```

Reviewer checklist
------------------

When reviewing code that touches tribles, look for these items:

- Does the code use find! to select only the fields it needs, rather than
  unfolding the entire graph?
- Are blob reads kept lazy (only read LongString when necessary)?
- Are push flows using the push/merge retry loop to avoid losing concurrent
  updates?
- Is the code avoiding holding the repo's Mutex across awaits and long
  blocking operations?
- Are optional fields handled explicitly (Option/Result)? Does the code
  fail loudly for required fields in critical paths?

Common pitfalls
---------------

- Folding the entire tribles graph into a single big Rust struct (expensive,
  brittle).
- Holding a Workspace/repo guard across async/await and triggering deadlocks.
- Assuming a field exists and panicking silently in production code.

Further reading and references
------------------------------

- See the tribles macros: attributes!, find!, pattern!, entity! in the tribles code
  for exact usage.
-- See your project's exporter/utilities for real implementations of persist
  and read flows; a centralized exporter reduces duplication in repo setup
  and tagging conventions.
- Type theory: "row polymorphism", "structural typing", "width subtyping"
  if you want the formal background.

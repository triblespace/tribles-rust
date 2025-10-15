# Formal Verification Roadmap

This roadmap captures the initial strategy for driving the `tribles` crates
toward comprehensive formal verification.  It unifies model checking, symbolic
execution, fuzzing, and deterministic simulation so we can reason about both the
low-level data structures and high-level repository workflows with stronger
correctness guarantees.

## Goals

- Protect the fundamental algebraic properties of `TribleSet`, `PATCH`, and the
  repository commit graph.
- Exercise serialization, deserialization, and zero-copy data views under
  adversarial inputs.
- Detect behavioural regressions in query planning, constraint solving, and
  workspace merging before they reach downstream users.
- Integrate the tooling into CI so proofs and regression checks run
  automatically for every change.

## Current Foundation

- `proofs/` already contains Kani harnesses for query, value, and variable-set
  behaviour.  They provide examples of bounded nondeterministic data generation
  (`kani::any`, `Value::new`) and assume/guarantee reasoning that new harnesses
  can reuse.
- `./scripts/preflight.sh` is the aggregation point for formatting and tests;
  adding verification steps here keeps contributor workflows consistent.

## Invariant Catalogue

The roadmap anchors future work around the following invariants.  Each row
tracks the subsystem we care about, the guarantees we want to encode, and a
rough sketch of how to exercise them in Kani, Miri, or fuzzing harnesses.

| Area | Key invariants | Candidate harness or check |
| --- | --- | --- |
| `TribleSet` (`src/trible/tribleset.rs`) | Union/intersection/difference maintain canonical ordering across all six PATCH indexes; iterators only yield deduplicated `Trible`s; `insert` never drops an ordering. | Extend the existing `variableset` harnesses with nondeterministic inserts, and add a dedicated `tribleset_harness.rs` validating round-trips across every ordering. |
| `PATCH` & `ByteTable` (`src/patch/*.rs`) | Cuckoo displacement respects `MAX_RETRIES` without losing entries; `Branch::modify_child` grows tables when required and preserves `leaf_count`/`segment_count`; `table_grow` copies every occupant exactly once. | Introduce a `patch_harness.rs` that stress-tests `plan_insert`, `table_insert`, and `Branch::grow`, plus a micro-fuzzer that drives inserts/removals across random table sizes. |
| Value schemas (`src/value/*.rs`) | Schema encoders respect declared byte widths; `Value::force` and `ValueSchema::validate` reject truncated buffers; zero-copy views stay aligned. | Reuse `value_harness.rs`, adding per-schema helpers plus a Miri regression suite that loads slices at every alignment. |
| Query engine (`src/query/*.rs`) | Constraint solver never aliases conflicting bindings; planner outputs cover all join permutations referenced by `pattern!`; influence tracking matches selected variables. | Expand `proofs/query_harness.rs` with minimal counterexamples, and fuzz constraint graphs via `cargo fuzz`. |
| Repository & commits (`src/repo/*.rs`, `proofs/commit_harness.rs`) | Branch heads remain append-only; `Workspace::pull` never forgets reachable blobs; selector algebra matches Git semantics. | Add bounded commit DAG generators in `commit_harness.rs` plus deterministic simulation traces covering merges and garbage collection. |
| Storage primitives (`src/blob`, `src/repo`, `src/patch/leaf.rs`) | Blob handles stay reference counted; pile headers remain within reserved capacity; byte slices from archives stay valid for the life of the store. | Combine Miri tests for aliasing with nightly fuzzers that replay repository sync transcripts. |

## Expansion Plan

### Phase 1 – Harden the Existing Kani Coverage

1. Catalogue crate-level invariants and map them to concrete Kani harnesses.
   Start with:
   - `TribleSet` operations preserving canonical ordering and deduplication.
   - Join planning in `atreides` ensuring variable bindings never alias
     conflicting values.
   - Repository merge logic maintaining append-only pile semantics.
2. Extract shared helpers for generating bounded arbitrary data (e.g.
   `Vec::bounded_any`) so harnesses remain expressive without exploding the
   search space.
3. Adopt a per-module harness layout (`proofs/<module>_harness.rs`) registered
   from `proofs/mod.rs` to make maintenance predictable.
4. Configure `scripts/verify.sh` to run targeted `cargo kani --harness <name>`
   invocations in parallel, then wire it into CI with caching to keep runtimes
   manageable.

### Phase 2 – Symbolic Execution with Miri

1. Enable `cargo miri test` for the default test suite to surface undefined
   behaviour (UB) and aliasing bugs that regular tests may miss.
2. Gate flaky or unsupported tests with `cfg(miri)` guards so the suite stays
   deterministic under the interpreter.
3. Document the workflow in `scripts/preflight.sh` and optionally expose a
   dedicated `scripts/miri.sh` for local runs when developers need deeper
   debugging.

### Phase 3 – Fuzzing and Property Testing

1. Introduce a `cargo fuzz` workspace targeting:
   - PATCH encoders/decoders with binary corpus seeds generated from integration
     tests.
   - Query planning to explore combinations of constraint graphs and filter
     predicates.
   - Repository sync workflows by fuzzing sequences of commits, pulls, and
     merges.
2. Reuse structured generators from `proptest` where deterministic shrinking is
   valuable, and bridge them into fuzz harnesses when possible to keep the state
   space constrained.
3. Automate nightly or on-demand fuzz campaigns via CI artifacts, storing any
   found counterexamples alongside minimised reproducers.

### Phase 4 – Deterministic Simulation Testing

1. Model repository replication scenarios with deterministic event queues to
   explore conflict resolution, garbage collection, and concurrent writers.
2. Encode the simulations as regular unit tests backed by recorded execution
   traces so they can double as documentation for expected behaviour.
3. Capture simulation scenarios discovered during fuzzing to prevent
   regressions.

## Tooling Integration

- Track verification status in CI badges and documentation so contributors know
  which guarantees currently hold.
- Extend `INVENTORY.md` with follow-up work items whenever new invariants or
  subsystems are identified.
- Keep verification-specific configuration (Kani property files, fuzz corpora,
  deterministic seeds) under version control to make runs reproducible.

## Next Steps

1. Break the invariant catalogue into GitHub issues that reference the planned
   harness files (`proofs/tribleset_harness.rs`, `proofs/patch_harness.rs`, etc.).
2. Prototype the PATCH harness that drives `Branch::modify_child` through
   insertion/growth cycles so we can assert the displacement planner and
   `table_grow` never drop entries; wire the run into `scripts/verify.sh`.
3. Evaluate CI capacity to determine how frequently Kani proofs, `cargo miri`,
   and fuzzers can run without blocking contributors, documenting the cadence
   directly in `INVENTORY.md`.

This roadmap should evolve alongside the codebase—update it whenever new
verification opportunities or obstacles appear.

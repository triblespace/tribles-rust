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

1. Finalise the invariant catalogue for Phase 1 and break it into actionable
   issues.
2. Prototype one additional Kani harness exercising PATCH serialisation to
   validate the workflow end-to-end.
3. Evaluate CI capacity to determine how frequently Kani proofs and fuzzers can
   run without blocking contributors.

This roadmap should evolve alongside the codebase—update it whenever new
verification opportunities or obstacles appear.

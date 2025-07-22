# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `INVENTORY.md` file and instructions for recording future work.
- README now links to the corresponding chapters on https://triblespace.github.io/tribles-rust.
- `branch_id_by_name` helper to resolve branch IDs from names. Returns a
  `NameConflict` error when multiple branches share the same name.
- Documentation and examples for the repository API.
- Test coverage for `branch_from` and `pull_with_key`.
- `Workspace::checkout` helper to load commit contents.
- `pattern!` now implemented as a procedural macro in the new `tribles-macros` crate.
- Expanded documentation for the `pattern` procedural macro to ease maintenance, including detailed comments inside the implementation.
- `EntityId` variants renamed to `Var` and `Lit` for consistency with field patterns.
- `Workspace::checkout` now accepts commit ranges for convenient history queries.
- Git-based terminology notes in the repository guide and a clearer workspace example.
- Expanded the repository example to store actual data and simplified the conflict loop.
- Documentation proposal for exposing blob metadata through the `Pile` API.
- `IndexEntry` now stores a timestamp for each blob. `PileReader::metadata`
  returns this timestamp along with the blob length.
- Repository workflows chapter covering branching, merging, CLI usage and an improved push/merge diagram.
- Separate `verify.sh` script for running Kani verification.
- Documented conflict resolution loop and clarified that returned workspaces
  contain updated metadata which must be pushed.
- Explained BranchStore's CAS-based optimistic concurrency control in the
  repository guide.
- Property tests for `ufoid` randomness and timestamp rollover.
- Further clarified `timestamp_distance` documentation that it only works with
- Documentation for built-in schemas and how to create your own.
  timestamps younger than the ~50-day rollover period.
- Added `HybridStore` to combine separate blob and branch stores.
- Added tests for the `ObjectStoreRemote` repository using the in-memory
  object store backend.
- Implemented `Debug` for `ObjectStoreRemote` and replaced `panic!` calls
  with `.expect()` in object store tests.
- Initial scaffold for a narrative "Tribles Book" documentation.
- Build script `build_book.sh` and CI workflow to publish the mdBook.
- Expanded the introduction and philosophy sections of the Tribles Book and
  documented how to install `mdbook`.
- Documented the pile file format in the book and expanded it with design rationale.
- Expanded the pile format chapter with recovery notes and a link to the `Pile` API docs.
- Added a book chapter describing the `find!` query language, listed
   built-in constraints, and included a reusable sample dataset for
   documentation examples.
- Added an architecture chapter that explains how `TribleSet` differs from the repository layer and details branch stores and commit flow. The diagram now better illustrates the commit flow.
- Added a "Developing Locally" chapter and linked it from the README and book introduction.
- Expanded the architecture chapter with design goals, semantic background and
  cross-references to other chapters.
- Clarified that the branch store's compare-and-set operation is the only
  place-oriented update, leaving the rest of the system value oriented and
  immutable.
- Documented the incremental query plan in `INVENTORY.md` and linked it
  to a new "Incremental Queries" book chapter detailing the approach.
- Noted that namespaces will expose a `delta!` operator, similar to
  `pattern!`, for expressing changes between `TribleSet`s. The macro
  computes the difference and uses `union!` internally to apply the
  delta constraint.
 - Documented potential commit selector redesign using git-style
   reachability semantics. Added a "Commit Selectors" design note with
    a table comparing Git syntax to the planned set-based API. The table
    is now exhaustive for Git's revision grammar, using only the general
    forms. Each entry links to the official documentation and marks
    selectors that are not planned for the initial implementation.
- Noted plans for a `delta!` operator to assist with incremental
  queries. Documentation describes how it will union patterns with
  each triple constrained to the dataset delta.
- Recorded a future task to generate namespaces from a TribleSet
  description and to rewrite `pattern!` as a procedural macro.
- Documented the internal `pattern_inner!` macro with expanded usage notes.
- Added inline comments for every `pattern_inner!` rule describing what it
  matches and why.
- Added a "PATCH" chapter to the book's deep dive section explaining the trie
  implementation.
- Recorded tasks to benchmark PATCH, analyze its algorithmic complexity and
  measure real-world space usage.

### Changed
- Removed the experimental `delta!` macro implementation; incremental
  query support will be revisited once `pattern!` becomes a procedural
  macro.
- Split branch lookup tests into independent cases for better readability.
- `Repository::checkout` was renamed to `pull` for symmetry with `push`.
- `IntoCheckoutRange` trait became `CommitSelector` and its `into_vec` method
  was renamed to `select`.
- Updated bucket handling to advance RNG state in `bucket_shove_random_slot`.
- Clarified need for duplicate `bucket_get_slot` check in `table_get_slot`.
- Replaced Elias--Fano arrays in `SuccinctArchive` with bit vectors for
  simpler builds and equivalent query performance.
- `SuccinctArchive` now counts distinct component pairs using bitsets,
  improving query estimation accuracy.
- Domain enumeration skips empty identifiers via `select0` and prefix bit
  vectors are constructed with `BitVector` for lower memory overhead.
- Improved `Debug` output for `Query` to show search state and bindings.
- Replaced branch allocation code with `Layout::from_size_align_unchecked`.
- Removed unused `FromBlob` and `TryToBlob` traits and updated documentation.
- Simplified constant comparison in query tests.
- `pattern!` now reuses attribute variables for identical field names.
- Clarified that the project's developer experience goal also includes
  providing an intuitive API for library users.
- Documented Kani proof guidelines to avoid constants and prefer
  `kani::any()` or bounded constructors for nondeterministic inputs.
- Fixed Kani playback build errors by using `dst_len` to access `child_table`
  length without implicit autorefs.
- Introduced `ValueSchema::validate` to verify raw value bit patterns.
- Query and value harnesses use this to avoid invalid `ShortString` data during playback.
- `ValueSchema::validate` now returns a `Result` and `Value::is_valid` provides
  a convenient boolean check.
- Corrected the workspace example to merge conflicts into the returned workspace
  and push that result.
- `preflight.sh` now only checks formatting and runs tests; Kani proofs run via `verify.sh`.
- Removed instruction to report unrelated Kani failures in PRs.
- Added missing documentation for several public structs and functions in
  `blob` and `repo` modules.
- Expanded the descriptions to clarify usage of public repository APIs.
- Moved repository and pile guides into module documentation and updated README links.
- Simplified toolchain setup. Scripts install `rustfmt` and `cargo-kani` via
  `cargo install` and rely on the system's default toolchain.
- Depend on the crates.io release `hifitime` 4.1.2 instead of the git repository.
- Added a README "Getting Started" section demonstrating `cargo add tribles` and
  a pile-backed repository example.
- Documented iteration order of `MemoryBlobStoreReader`, noted workspace use of
  `MemoryBlobStore::new` and improved `Pile::try_open` description.
- Restricted `PileSwap` and `PileAux` to crate visibility.
- Repository guidelines now discourage asynchronous code in favor of
  synchronous implementations that can be parallelized.
- Renamed `ObjectStoreRepo` to `ObjectStoreRemote` in the object-store backend.
- Listing iterators for the object-store backend now stream directly from the
  underlying store instead of collecting results in memory.
- `Repository::push` now returns `Option<Workspace>` instead of the custom
  `RepoPushResult` enum, simplifying conflict handling.
- Split identifier and trible structure discussions into dedicated deep-dive book chapters.
- `preflight.sh` now verifies that the mdBook documentation builds successfully.
- Fixed book `SUMMARY.md` so preflight passes without parse errors.
- `Workspace` now exposes a `put` method for adding blobs, replacing the old
  `add_blob` helper. The method returns the stored blob's handle directly since
  the underlying store cannot fail.
- `Workspace::get` method retrieves blobs from the local store and falls back to
  the base store when needed.
- `OpenError` now implements `std::error::Error` and provides clearer messages when opening piles.
- Removed the `..=` commit range selector. The `..` selector now follows Git's
  semantics and excludes the starting commit.
- Extracted `collect_range` into a standalone function for clarity.
- Moved `first_parent` into a standalone function for clarity.
- Added a `collect_reachable` helper to gather all commits reachable from a
  starting point.
- Scalar commit selectors once again return only the specified commit.
- Introduced an `ancestors` selector to retrieve a commit and its history.
- Commit selectors now return a `CommitSet` patch of commit handles instead of a `Vec`.
- Renamed the `CommitPatch` type alias to `CommitSet`.

## [0.5.2] - 2025-06-30
### Added
- Initial changelog file.
- Repository guidelines now require documenting tasks in `CHANGELOG.md`.
- Converted object-store backend to `BranchStore`/`BlobStore` API.


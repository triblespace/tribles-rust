# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Documentation and examples for the repository API.
- Test coverage for `branch_from` and `checkout_with_key`.
- Git-based terminology notes in the repository guide and a clearer workspace example.
- Expanded the repository example to store actual data and simplified the conflict loop.
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

### Changed
- Updated bucket handling to advance RNG state in `bucket_shove_random_slot`.
- Clarified need for duplicate `bucket_get_slot` check in `table_get_slot`.
- Replaced Elias--Fano arrays in `SuccinctArchive` with bit vectors for
  simpler builds and equivalent query performance.
- `SuccinctArchive` now counts distinct component pairs using bitsets,
  improving query estimation accuracy.
- Domain enumeration skips empty identifiers via `select0` and prefix bit
  vectors are constructed with `BitVec` for lower memory overhead.
- Improved `Debug` output for `Query` to show search state and bindings.
- Replaced branch allocation code with `Layout::from_size_align_unchecked`.
- Removed unused `FromBlob` and `TryToBlob` traits and updated documentation.
- Simplified constant comparison in query tests.
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

## [0.5.2] - 2025-06-30
### Added
- Initial changelog file.
- Repository guidelines now require documenting tasks in `CHANGELOG.md`.
- Converted object-store backend to `BranchStore`/`BlobStore` API.


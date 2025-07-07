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
- Separate `verify.sh` script for running Kani verification.
- Documented conflict resolution loop and clarified that returned workspaces
  contain updated metadata which must be pushed.
- Explained BranchStore's CAS-based optimistic concurrency control in the
  repository guide.

### Changed
- Updated bucket handling to advance RNG state in `bucket_shove_random_slot`.
- Clarified need for duplicate `bucket_get_slot` check in `table_get_slot`.
- `SuccinctArchive` now counts distinct component pairs using bitsets,
  improving query estimation accuracy.
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
- Moved repository and pile guides into module documentation and updated README links.

## [0.5.2] - 2025-06-30
### Added
- Initial changelog file.
- Repository guidelines now require documenting tasks in `CHANGELOG.md`.


# Documentation Improvement Ideas

Reading through the crate source highlighted a few topics that deserve their own
sections in the book. These notes capture the main gaps so future revisions can
prioritise the most useful additions.

- **Remote object stores** &mdash; `repo::objectstore::ObjectStoreRemote::with_url`
  wires the repository into [`object_store`](https://docs.rs/object_store/latest/object_store/)
  services such as S3, local filesystems or Azure storage. A step-by-step guide
  should show how to configure credentials, pick a prefix and combine the remote
  backend with other stores (for example via `HybridStore`). The chapter should
  also spell out how branch updates rely on `PutMode::Update`/`UpdateVersion`
  retries, how conflicts are surfaced, and how blob and branch listings are
  streamed through the `BlockingIter` adapter so users know what consistency
  guarantees to expect.【F:src/repo/objectstore.rs†L108-L316】
- **Hybrid storage recipes** &mdash; The `repo::hybridstore::HybridStore` adapter
  mixes a blob store with a separate branch store. Documenting common layouts
  (remote blobs + local branches, or piles + in-memory branches) would help
  readers choose a deployment pattern quickly.
- **Signature verification** &mdash; Both `repo::commit::verify` and
  `repo::branch::verify` expose helpers to validate the signed metadata before
  accepting remote history. A short example walking through verification before
  merging would make the security model clearer.
- **Repository migration helpers** &mdash; `repo::transfer` rewrites whichever
  handles you feed it, returning the old and new identifiers so callers can
  update references. A migration recipe could show how to collect handles from
  `BlobStoreList::blobs()` for full copies or from `reachable` when only live
  data should be duplicated.
- **Conservative GC tooling** &mdash; The garbage-collection chapter already covers
  the high-level approach, but it could reference concrete APIs such as
  `repo::reachable`, `repo::transfer`, and `MemoryBlobStore::keep` to show how to
  compute and retain the reachable set in code.
- **Succinct archive indexes** &mdash; `blob::schemas::succinctarchive::SuccinctArchive`
  converts a `TribleSet` into compressed wavelet matrices, exposes helpers such
  as `distinct_in`/`enumerate_in`, implements `TriblePattern` and serialises via
  ordered, compressed or cached `Universe` implementations. A dedicated section
  should walk through building an archive from a set, choosing a universe,
  storing it as a blob and querying it directly through
  `SuccinctArchiveConstraint` so readers can reuse the on-disk index without
  round-tripping through `TribleSet` conversions.【F:src/blob/schemas/succinctarchive.rs†L100-L529】【F:src/blob/schemas/succinctarchive/universe.rs†L16-L265】【F:src/blob/schemas/succinctarchive/succinctarchiveconstraint.rs†L9-L200】
- **Extensible path engines** &mdash; Regular path queries run through
  `RegularPathConstraint` which delegates edge checks to the `PathEngine` trait.
  Document how the built-in `ThompsonEngine` constructs NFAs from a `TribleSet`
  and show how to plug in alternative engines backed by other graph stores so
  readers can extend the regex-based traversal beyond in-memory datasets.【F:src/query/regularpathconstraint.rs†L1-L200】

Treat these bullets as a living backlog for book improvements. As the
implementation evolves, refresh the list so the documentation keeps pace with
new capabilities.

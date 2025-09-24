# Documentation Improvement Ideas

Reading through the crate source highlighted a few topics that deserve their own
sections in the book. These notes capture the main gaps so future revisions can
prioritise the most useful additions.

- **Remote object stores** &mdash; `repo::objectstore::ObjectStoreRemote::with_url`
  wires the repository into [`object_store`](https://docs.rs/object_store/latest/object_store/)
  services such as S3, local filesystems or Azure storage. A step-by-step guide
  should show how to configure credentials, pick a prefix and combine the remote
  backend with other stores (for example via `HybridStore`).
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

Treat these bullets as a living backlog for book improvements. As the
implementation evolves, refresh the list so the documentation keeps pace with
new capabilities.

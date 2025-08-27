# Pile Blob Metadata

The pile file already stores a timestamp and length in each blob header but this
information is discarded when building the in-memory index. Clients therefore
cannot query when a blob was added or how large it is without re-parsing the
file.

## Proposed Changes

- Extend `IndexEntry` in `src/repo/pile.rs` with a `timestamp` field. The length
  can be determined from the stored bytes when needed.
- Introduce a public `BlobMetadata` struct containing `timestamp` and `length`
  so callers do not depend on internal types.
- Populate the timestamp when `Pile::refresh` scans existing entries and when
  inserting new blobs. Lengths are computed on demand.
- Add `PileReader::metadata(&self, handle)` to retrieve a blob's metadata if it
  exists. Iterators may later be extended to yield this information alongside the
  blob itself.

This approach keeps the current API intact while making useful details available
for replication and debugging tools.

# Garbage Collection and Forgetting

Repositories grow over time as commits, branch metadata and user blobs
accumulate. Because each blob is content addressed and immutable, nothing is
ever overwritten and there is no automatic reclamation when branches move or
objects become orphaned. To keep disk usage in check a repository can
periodically _forget_ blobs that are no longer referenced. Forgetting only
removes local copies; any blob can be reintroduced later without violating
TribleSpace's monotonic model. The challenge is deciding which blobs are still
reachable without rebuilding every record.

## Conservative Reachability

Every commit and branch metadata record is stored as a `SimpleArchive`. The
archive encodes a canonical `TribleSet` as alternating 32‑byte keys and values.
Instead of deserialising the archive, the collector scans the raw bytes in
32‑byte chunks. Each second chunk is treated as a candidate value. If a chunk
matches the hash of a blob in the store we assume it is a reference, regardless
of the attribute type. With 32‑byte hashes the odds of a random collision are
negligible, so the scan may keep extra blobs but will not drop a referenced one.

## Traversal Algorithm

1. Enumerate all branches and load their metadata blobs.
2. Extract candidate handles from the metadata. This reveals the current commit
   head along with any other referenced blobs.
3. Recursively walk the discovered commits and content blobs. Whenever a
   referenced blob is a `SimpleArchive`, scan every second 32‑byte segment for
   further handles instead of deserialising it.
4. Collect all visited handles into a plain set or list of 32‑byte handles.
   A `keep`‑style operation can pass this collection to the blob store and
   prune everything else without imposing any trible semantics.

Content blobs that are not `SimpleArchive` instances (for example large binary
attachments) act as leaves. They become reachable when some archive references
their handle and are otherwise eligible for forgetting.

## Future Work

The public API for triggering garbage collection is still open. The blob store
could expose a method that retains only a supplied collection of handles, or a
helper such as `Repository::forget_unreachable()` might compute those handles
before delegating pruning. A more flexible `ReachabilityWalker` could also let
applications decide how to handle reachable blobs. Whatever interface emerges,
conservative reachability by scanning `SimpleArchive` bytes lays the groundwork
for safe space reclamation.

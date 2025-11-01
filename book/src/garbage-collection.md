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
4. Stream the discovered handles into whatever operation you need. The
   [`reachable`](https://docs.rs/triblespace/latest/triblespace/repo/fn.reachable.html)
   helper returns an iterator of handles, so you can retain them, transfer
   them into another store, or collect them into whichever structure your
   workflow expects.

Content blobs that are not `SimpleArchive` instances (for example large binary
attachments) act as leaves. They become reachable when some archive references
their handle and are otherwise eligible for forgetting.

## Automating the Walk

The repository module already provides most of the required plumbing. The
[`reachable`](https://docs.rs/triblespace/latest/triblespace/repo/fn.reachable.html)
helper exposes the traversal as a reusable iterator so you can compose other
operations along the way, while
[`transfer`](https://docs.rs/triblespace/latest/triblespace/repo/fn.transfer.html)
duplicates whichever handles you feed it. The in-memory `MemoryBlobStore` can
retain live blobs, duplicate them into a scratch store, and report how many
handles were touched without writing bespoke walkers:

```rust
use triblespace::blob::memoryblobstore::MemoryBlobStore;
use triblespace::repo::{self, BlobStoreKeep, BlobStoreList};
use triblespace::value::schemas::hash::Blake3;

let mut store = MemoryBlobStore::<Blake3>::default();
// ... populate the store or import data ...

let reader = store.reader()?;
let roots = reader.blobs().collect::<Result<Vec<_>, _>>()?;

// Trim unreachable blobs in-place.
store.keep(repo::reachable(&reader, roots.clone()));

// Optionally copy the same reachable blobs into another store.
let mut scratch = MemoryBlobStore::<Blake3>::default();
let visited = repo::reachable(&reader, roots.clone()).count();
let mapping: Vec<_> = repo::transfer(
    &reader,
    &mut scratch,
    repo::reachable(&reader, roots),
)
.collect::<Result<_, _>>()?;

println!("visited {} blobs, copied {}", visited, mapping.len());
println!("rewrote {} handles", mapping.len());
```

Every blob store reader implements the
[`BlobStoreList`](https://docs.rs/triblespace/latest/triblespace/repo/trait.BlobStoreList.html)
trait, which exposes helpers such as `blobs()` for enumerating stored handles.
In practice you would seed the walker with the handles extracted from branch
metadata or other root sets instead of iterating the entire store. The helper
takes any `IntoIterator` of handles, so once branch heads (and other roots) have
been identified, they can be fed directly into the traversal without writing
custom queues or visitor logic. Passing the resulting iterator to
`MemoryBlobStore::keep` or `repo::transfer` makes it easy to implement
mark-and-sweep collectors or selective replication pipelines without duplicating
traversal code.

When you already have metadata represented as a `TribleSet`, the
[`potential_handles`](https://docs.rs/triblespace/latest/triblespace/repo/fn.potential_handles.html)
helper converts its value column into the conservative stream of
`Handle<H, UnknownBlob>` instances expected by these operations.

## Future Work

The public API for triggering garbage collection is still evolving. The
composition-friendly walker introduced above is one building block; future work
could layer additional convenience helpers or integrate with external retention
policies. Conservative reachability by scanning `SimpleArchive` bytes remains
the foundation for safe space reclamation.

# Garbage Collection and Forgetting

Repositories grow over time as new blobs are written for commits, branch
metadata and user data.  Because each blob is content addressed there is no
built‑in mechanism to reclaim space when branches move or objects become
orphaned.  A repository may choose to _forget_ unreachable blobs to recover
storage.  Forgetting does not violate TribleSpace's monotonic model – removed
blobs can always be reintroduced if discovered again – but it requires a
conservative reachability analysis to avoid discarding data still referenced by
existing branches.

## Conservative Reachability

Every commit and branch metadata record is stored as a `SimpleArchive`.  The
format is a canonicalised `TribleSet` encoded as alternating 32‑byte keys and
values.  When searching for references we do not rebuild the set.  Instead we
scan the archive in 32‑byte chunks and treat every second segment as a candidate
value.  Any segment that matches the hash of a blob in the store is considered a
potential handle.  The type of the attribute is irrelevant; if the bytes happen
to equal a known handle we assume a reference exists.  With 32‑byte hashes the
chance of a random collision is vanishingly small, so a match strongly implies a
real reference.  This heuristic may keep more blobs than strictly necessary but
never frees ones that are still in use.

## Traversal Algorithm

1. Enumerate all branches and load their metadata blobs.
2. For each metadata blob extract values that look like blob handles.  This
   yields the current commit head as well as any other referenced blobs.
3. Recursively walk commit and content blobs discovered this way.  Whenever a
   referenced blob is a `SimpleArchive`, scan every second 32‑byte segment for
   handles instead of deserialising the entire set.
4. Collect every handle visited during the walk into a `TribleSet`.  Repositories
   that implement a `keep`‑style operation can feed this set back to the blob
   store to drop everything else.

Content blobs that are not `SimpleArchive` instances (for example large binary
attachments) are treated as leaves.  They become reachable when some archive
contains their handle and are otherwise eligible for forgetting.

## Future Work

The exact API for triggering garbage collection is still open.  One option is a
`Repository::forget_unreachable()` helper that relies on the blob store to
implement a pruning method.  Another approach could expose a generic
`ReachabilityWalker` so applications can decide how to handle or export reachable
blobs.  Regardless of the final interface, conservative reachability based on
scanning `SimpleArchive` bytes lays the groundwork for safe space reclamation.

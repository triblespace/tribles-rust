# Repository Workflows

Tribles borrows much of its vocabulary from Git:

* **Repository** – top-level object that tracks history through a `BlobStore`
  and `BranchStore`.
* **Workspace** – mutable view of a branch, similar to Git's working directory
  and index combined.
* **BlobStore** – stores commits and blobs.
* **BranchStore** – records branch metadata.

Both stores can be in memory, on disk or backed by a remote service. The
examples in `examples/repo.rs` and `examples/workspace.rs` showcase this API and
should feel familiar to anyone comfortable with Git.

## Storage Backends and Composition

`Repository` accepts any storage that implements both the `BlobStore` and
`BranchStore` traits, so you can combine backends to fit your deployment. The
crate ships with a few ready-made options:

- [`MemoryRepo`](../src/repo/memoryrepo.rs) stores everything in memory and is
  ideal for tests or short-lived tooling where persistence is optional.
- [`Pile`](../src/repo/pile.rs) persists blobs and branch metadata in a single
  append-only file. It is the default choice for durable local repositories and
  integrates with the pile tooling described in [Pile Format](pile-format.md).
- [`ObjectStoreRemote`](../src/repo/objectstore.rs) connects to
  [`object_store`](https://docs.rs/object_store/latest/object_store/) endpoints
  (S3, local filesystems, etc.). It keeps all repository data in the remote
  service and is useful when you want a shared blob store without running a
  dedicated server.
- [`HybridStore`](../src/repo/hybridstore.rs) lets you split responsibilities,
  e.g. storing blobs on disk while keeping branch heads in memory or another
  backend. Any combination that satisfies the trait bounds works.

Backends that need explicit shutdown can implement `StorageClose`. When the
repository type exposes that trait bound you can call `repo.close()?` to flush
and release resources instead of relying on `Drop` to run at an unknown time.

```rust,ignore
use tribles::repo::hybridstore::HybridStore;
use tribles::repo::memoryrepo::MemoryRepo;
use tribles::repo::objectstore::ObjectStoreRemote;
use tribles::repo::Repository;
use tribles::value::schemas::hash::Blake3;
use url::Url;

let blob_remote: ObjectStoreRemote<Blake3> =
    ObjectStoreRemote::with_url(&Url::parse("s3://bucket/prefix")?)?;
let branch_store = MemoryRepo::default();
let storage = HybridStore::new(blob_remote, branch_store);
let mut repo = Repository::new(storage, signing_key);

// Work with repo as usual …
// repo.close()?; // if the underlying storage supports StorageClose
```

## Branching

A branch records a line of history. Creating one writes initial metadata to the
underlying store and returns an `ExclusiveId` guard for that branch. Pulling the
branch yields a `Workspace` pointing at its head. Typical steps
look like:

1. Create a repository backed by blob and branch stores.
2. Create or look up a branch ID, then call `Repository::pull` to obtain a
   `Workspace`.
3. Commit changes in the workspace.
4. Push the workspace to publish those commits.

Use `Repository::create_branch` to initialize new branches. It returns an
`ExclusiveId` guard that ensures the ID is not simultaneously handed out to
another writer. Pass the guard's underlying `Id` to `Repository::pull` to obtain
workspaces for that branch, and drop or `release` the guard once exclusive
ownership is no longer required:

```rust
let mut repo = Repository::new(pile, SigningKey::generate(&mut OsRng));
let branch_id = repo.create_branch("main", None).expect("create branch");
let mut ws = repo.pull(*branch_id).expect("open branch");
let mut ws2 = repo.pull(ws.branch_id()).expect("open branch");
```

After committing changes you can push the workspace back:

```rust
ws.commit(change, Some("initial commit"));
repo.push(&mut ws)?;
```

## Inspecting History

You can explore previous commits using `Workspace::checkout` which returns a
`TribleSet` with the union of the specified commit contents. Passing a single
commit returns just that commit. To include its history you can use the
`ancestors` helper. Commit ranges are supported for convenience. The expression
`a..b` yields every commit reachable from `b` that is not reachable from `a`,
treating missing endpoints as empty (`..b`) or the current `HEAD` (`a..` and
`..`):

```rust
let history = ws.checkout(commit_a..commit_b)?;
let full = ws.checkout(ancestors(commit_b))?;
```

The [`history_of`](../src/repo.rs) helper builds on the `filter` selector to
retrieve only the commits affecting a specific entity. Commit selectors are
covered in more detail in the next chapter:

```rust
let entity_changes = ws.checkout(history_of(my_entity))?;
```

## Merging and Conflict Handling

When pushing a workspace another client might have already updated the branch.
`Repository::push` attempts to update the branch atomically and returns an
optional conflicting `Workspace` if the head moved. The usual loop looks like:

```rust
while let Some(mut incoming) = repo.push(&mut ws)? {
    incoming.merge(&mut ws)?;
    ws = incoming;
}
```

After a successful push the branch may have advanced further than the head
supplied, because the repository refreshes its view after releasing the lock.
An error indicating a corrupted pile does not necessarily mean the push failed;
the update might have been written before the corruption occurred.

This snippet is taken from [`examples/workspace.rs`](../examples/workspace.rs).
The [`examples/repo.rs`](../examples/repo.rs) example demonstrates the same
pattern with two separate workspaces. The returned `Workspace` already contains
the remote commits, so after merging your changes you push that new workspace to
continue.

## Typical CLI Usage

There is a small command line front-end in the
[`trible`](https://github.com/triblespace/trible) repository. It exposes push
and merge operations over simple commands and follows the same API presented in
the examples. The tool is currently experimental and may lag behind the library,
but it demonstrates how repository operations map onto a CLI.

## Diagram

A simplified view of the push/merge cycle:

```text

        ┌───────────┐         pull          ┌───────────┐
        | local ws  |◀───────────────────── |   repo    |
        └─────┬─────┘                       └───────────┘
              │
              │ commit
              │                                                                      
              ▼                                   
        ┌───────────┐         push          ┌───────────┐
        │  local ws │ ─────────────────────▶│   repo    │
        └─────┬─────┘                       └─────┬─────┘
              │                                   │
              │ merge                             │ conflict?
              └──────▶┌─────────────┐◀────────────┘
                      │ conflict ws │       
                      └───────┬─────┘
                              │             ┌───────────┐
                              └────────────▶|   repo    │
                                     push   └───────────┘
   
```

Each push either succeeds or returns a workspace containing the other changes.
Merging incorporates your commits and the process repeats until no conflicts
remain.

## Attaching a Foreign History (merge-import)

Sometimes you want to graft an existing branch from another pile into your
current repository without rewriting its commits. Tribles supports a
conservative, schema‑agnostic import followed by a single merge commit:

1. Copy all reachable blobs from the source branch head into the target pile
   using `copy_reachable`, which walks every 32‑byte aligned chunk in each
   blob and enqueues any candidate that dereferences in the source.
2. Create a single merge commit that has two parents: your current branch head
   and the imported head. No content is attached to the merge; it simply ties
   the DAGs together.

This yields a faithful attachment of the foreign history — commits and their
content are copied verbatim, and a one‑off merge connects both histories.

The `trible` CLI exposes this as:

```sh
trible branch merge-import \
  --from-pile /path/to/src.pile --from-name source-branch \
  --to-pile   /path/to/dst.pile --to-name   self
```

Internally this uses `repo::copy_reachable` and `Workspace::merge_commit`.
Because `copy_reachable` scans aligned 32‑byte chunks, it is forward‑compatible
with new formats as long as embedded handles remain 32‑aligned.

### Programmatic example (Rust)

The same flow can be used directly from Rust when you have two piles on disk and
want to attach the history of one branch to another:

```rust,ignore
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tribles::prelude::*;
use tribles::repo::pile::Pile;
use tribles::repo::Repository;
use tribles::value::schemas::hash::Blake3;
use tribles::value::schemas::hash::Handle;

fn merge_import_example(
    src_path: &std::path::Path,
    src_branch_id: tribles::id::Id,
    dst_path: &std::path::Path,
    dst_branch_id: tribles::id::Id,
) -> anyhow::Result<()> {
    // 1) Open source (read) and destination (write) piles
    let mut src = Pile::open(src_path)?;
    src.restore()?;
    let mut dst = Pile::open(dst_path)?;
    dst.restore()?;

    // 2) Resolve source head commit handle
    let src_head: Value<Handle<Blake3, blobschemas::SimpleArchive>> =
        src.head(src_branch_id)?.ok_or_else(|| anyhow::anyhow!("source head not found"))?;

    // 3) Conservatively copy all reachable blobs from source → destination
    let stats = repo::copy_reachable(&src.reader()?, &mut dst, [src_head.transmute()])?;
    eprintln!("copied: visited={} stored={}", stats.visited, stats.stored);

    // 4) Attach via a single merge commit in the destination branch
    let mut repo = Repository::new(dst, SigningKey::generate(&mut OsRng));
    let mut ws = repo.pull(dst_branch_id)?;
    ws.merge_commit(src_head)?; // parents = { current HEAD, src_head }

    // 5) Push with standard conflict resolution
    while let Some(mut incoming) = repo.push(&mut ws)? {
        incoming.merge(&mut ws)?;
        ws = incoming;
    }
    Ok(())
}
```

# Architecture Overview

Trible Space consists of two layers.  At the core is [`TribleSet`](https://docs.rs/tribles/latest/tribles/trible/struct.TribleSet.html), an in‑memory data structure that behaves much like a hashmap.  It is cheap to create and merge and can be used on its own when durability is not required.  The optional repository layer stores those trible sets as blobs and links them through commits so that data can be exchanged and shared across machines.

The sections below outline how [`tribles::repo`](https://docs.rs/tribles/latest/tribles/repo/index.html) combines blob and branch stores into repositories and workspaces.

## Trible Sets

[`Trible`](https://docs.rs/tribles/latest/tribles/trible/struct.Trible.html) values are stored in `TribleSet`s that function much like keyed hashmaps. They provide efficient querying and merging without any external storage. When you want to persist or exchange a set, it can be serialized into a blob and tracked by the repository layer.

## Blob Storage

Every byte of data is placed in a [`BlobStore`](https://docs.rs/tribles/latest/tribles/blob/index.html).  The trait abstracts the backing implementation so the same code works with an in‑memory [`MemoryBlobStore`](https://docs.rs/tribles/latest/tribles/blob/struct.MemoryBlobStore.html), an on‑disk [`Pile`](https://docs.rs/tribles/latest/tribles/repo/pile/struct.Pile.html) or a remote object store.  Trible sets, commit records and arbitrary user blobs are all inserted via `put` and addressed by their hash.

## Branch Store

Repositories keep track of branch heads in a [`BranchStore`](https://docs.rs/tribles/latest/tribles/repo/trait.BranchStore.html).  The store maps branch identifiers to the latest commit and uses a simple compare‑and‑set update to avoid conflicts.  Pile and the in‑memory repo both provide branch store implementations.

## Repository

The [`Repository`](https://docs.rs/tribles/latest/tribles/repo/struct.Repository.html) combines a blob store with a branch store and exposes higher level operations similar to a remote Git repository.  Commits reference blobs holding the changed [`TribleSet`](https://docs.rs/tribles/latest/tribles/trible/struct.TribleSet.html) and optionally point to a parent commit.

## Workspaces

A [`Workspace`](https://docs.rs/tribles/latest/tribles/repo/struct.Workspace.html) represents mutable state during editing.  When you `branch` or `checkout` you receive a workspace with a fresh [`MemoryBlobStore`](https://docs.rs/tribles/latest/tribles/blob/struct.MemoryBlobStore.html) for new blobs.  Commits created in the workspace are stored locally until `push` updates the repository's branch store.  Multiple workspaces can be merged before pushing to resolve conflicts.

## Commits and History

[`Trible`](https://docs.rs/tribles/latest/tribles/trible/struct.Trible.html) is the smallest unit of information. `TribleSet`s can be written to a blob and committed to create an immutable history.  Each commit links to the previous one and is signed by the author.  This chain forms the durable database layer that repositories expose.

### Putting It Together

```text
+-----------------------------------------------------------+
|                        Repository                          |
|   +---------------------+   +----------------------------+ |
|   |      BlobStore      |   |        BranchStore        | |
|   +---------------------+   +----------------------------+ |
+-----------------------------------------------------------+
           ^ checkout                            | push
           |                                     v
+-----------------------------------------------------------+
|                        Workspace                           |
|   +---------------------+   +----------------------------+ |
|   |   MemoryBlobStore   |   |          TribleSet        | |
|   +---------------------+   +----------------------------+ |
+-----------------------------------------------------------+
                      |
                      | commit/add_blob
                      v
                 TribleSet blobs
```

Repositories persist blobs and branch metadata, while workspaces stage changes before pushing them.  Because everything is content addressed, different repositories can easily share blobs through a common object store.

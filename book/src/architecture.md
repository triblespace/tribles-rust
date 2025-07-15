# Architecture Overview

Trible Space is designed to keep data management simple, safe and fast.  The [README](../README.md) introduces these goals in more detail, emphasizing a lean design with predictable performance and straightforward developer experience.  This chapter explains how the pieces fit together and why they are organised this way.

## Design Goals

A full discussion of the motivation behind Trible Space can be found in the [Philosophy](deep-dive/philosophy.md) section.  At a high level we want a self‑contained data store that offers:

- **Simplicity** – minimal moving parts and predictable behaviour.
- **Developer Experience** – a clear API that avoids complex servers or background processes.
- **Safety and Performance** – sound data structures backed by efficient content addressed blobs.

These goals grew out of earlier "semantic" technologies that attempted to model knowledge as graphs.  While systems like RDF promised great flexibility, in practice they often became difficult to host, query and synchronise.  Trible Space keeps the idea of describing the world with simple statements but stores them in a form that is easy to exchange and reason about.

## Data Model

The fundamental unit of information is a [`Trible`](https://docs.rs/tribles/latest/tribles/trible/struct.Trible.html).  Its 64 byte layout is described in [Trible Structure](deep-dive/trible-structure.md).  A `Trible` links a subject entity to an attribute and value.  Multiple tribles are stored in a [`TribleSet`](https://docs.rs/tribles/latest/tribles/trible/struct.TribleSet.html), which behaves like a hashmap with three columns — subject, attribute and value.

## Trible Sets

`TribleSet`s provide fast querying and cheap copy‑on‑write semantics.  They can be merged, diffed and searched entirely in memory.  When durability is needed the set is serialised into a blob and tracked by the repository layer.

## Blob Storage

All persistent data lives in a [`BlobStore`](https://docs.rs/tribles/latest/tribles/blob/index.html).  Every blob is addressed by the hash of its contents.  This content addressing means the same data is never stored twice and its integrity can be verified on read.  Different implementations handle where bytes actually reside: an in‑memory [`MemoryBlobStore`](https://docs.rs/tribles/latest/tribles/blob/struct.MemoryBlobStore.html), an on‑disk [`Pile`](https://docs.rs/tribles/latest/tribles/repo/pile/struct.Pile.html) described in [Pile Format](pile-format.md) or a remote object store.  Trible sets, user blobs and commit records all share this mechanism.

## Branch Store

A [`BranchStore`](https://docs.rs/tribles/latest/tribles/repo/trait.BranchStore.html) keeps track of the tips of each branch.  Updates use a simple compare‑and‑set operation so concurrent writers detect conflicts.  Both the in‑memory and pile repositories implement this trait.

Because only this single operation mutates repository state, nearly all other logic is value oriented and immutable.  Conflicts surface only at the branch store update step, which simplifies concurrent use and reasoning about changes.

## Repository

The [`Repository`](https://docs.rs/tribles/latest/tribles/repo/struct.Repository.html) combines a blob store with a branch store.  Commits store a trible set blob along with a parent link and signature.  Because everything is content addressed, multiple repositories can share blobs or synchronize through a basic file copy.

## Workspaces

A [`Workspace`](https://docs.rs/tribles/latest/tribles/repo/struct.Workspace.html) represents mutable state during editing.  Checking out or branching yields a workspace backed by a fresh `MemoryBlobStore`.  Commits are created locally and only become visible to others when pushed, as described in [Repository Workflows](repository-workflows.md).

## Commits and History

`TribleSet`s written to blobs form immutable commits.  Each commit references its parent, creating an append‑only chain signed by the author.  This is the durable history shared between repositories.

## Putting It Together

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

Repositories persist blobs and branch metadata while workspaces stage changes before pushing them.  Because every blob is addressed by its hash, repositories can safely share data through any common storage without coordination.

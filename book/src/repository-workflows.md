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

## Branching

A branch records a line of history. Creating one writes initial metadata to the
underlying store and yields a `Workspace` pointing at that branch. Typical steps
look like:

1. Create a repository backed by blob and branch stores.
2. Open or create a branch to obtain a `Workspace`.
3. Commit changes in the workspace.
4. Push the workspace to publish those commits.

While `Repository::branch` is a convenient way to start a fresh branch, most
workflows use `Repository::pull` to obtain a workspace for an existing branch:

```rust
let mut repo = Repository::new(pile, SigningKey::generate(&mut OsRng));
let mut ws = repo.branch("main").expect("create branch");
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

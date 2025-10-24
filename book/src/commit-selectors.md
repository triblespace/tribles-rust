# Commit Selectors

Commit selectors describe which commits to load from a workspace. The
`Workspace::checkout` method accepts any type implementing the
`CommitSelector` trait and returns a `TribleSet` containing data from those
commits. It currently supports individual commit handles, lists of handles and a
handful of higher level selectors.

Most selectors operate on ranges inspired by Git's two‑dot syntax. `a..b`
walks the parents reachable from `b` and stops descending a branch once it
encounters a commit selected by `a`. Each boundary is exclusive, so the
boundary commits themselves are omitted while ancestors reachable through other
paths remain visible. Omitting the start defaults `a` to an empty selector,
making `..b` walk all ancestors of `b`. Omitting the end defaults `b` to the
current `HEAD`, so `a..` gathers history from `HEAD` until the walk reaches `a`
and `..` expands to the full ancestor chain of `HEAD`.

To reproduce Git's set-difference semantics, wrap the boundary in `ancestors`:
`ancestors(a)..b` behaves like `git log a..b`.

```rust
// Check out the entire history of the current branch
let history = ws.checkout(ancestors(ws.head()))?;
```

While convenient, the range-based design makes it difficult to compose complex
queries over the commit graph.

## Implemented selectors

`CommitSelector` is implemented for:

- `CommitHandle` – a single commit.
- `Vec<CommitHandle>` and `&[CommitHandle]` – explicit lists of commits.
- `ancestors(commit)` – a commit and all of its ancestors.
- `nth_ancestor(commit, n)` – follows the first-parent chain `n` steps.
- `parents(commit)` – direct parents of a commit.
- `symmetric_diff(a, b)` – commits reachable from either `a` or `b` but not
  both.
- Standard ranges: `a..b`, `a..`, `..b` and `..` that stop walking once the
  start boundary is encountered.
- `filter(selector, predicate)` – retains commits for which `predicate`
  returns `true`.
- `history_of(entity)` – commits touching a specific entity (built on
  `filter`).
- `time_range(start, end)` – commits whose timestamps intersect the inclusive
  range.

A future redesign could mirror Git's revision selection semantics.
Instead of passing ranges, callers would construct *commit sets* derived from
reachability.  Primitive functions like `ancestors(<commit>)` and
`descendants(<commit>)` would produce sets.  Higher level combinators such as
`union`, `intersection` and `difference` would then let users express queries
like "A minus B" or "ancestors of A intersect B".  Each selector would return
a `CommitSet` patch of commit handles for `checkout` to load.

This approach aligns with Git's mental model and keeps selection logic separate
from workspace mutation.  It also opens the door for additional operations on
commit sets without complicating the core API.

## Filtering commits

The `filter` selector wraps another selector and keeps only the commits for
which a user provided closure returns `true`. The closure receives the commit
metadata and its payload, allowing inspection of authors, timestamps or the
data itself. Selectors compose, so you can further narrow a range:

```rust
use hifitime::Epoch;
use triblespace::repo::{filter, time_range};

let since = Epoch::from_unix_seconds(1_609_459_200.0); // 2020-12-01
let now = Epoch::now().unwrap();
let recent = ws.checkout(filter(time_range(since, now), |_, payload| {
    payload.iter().any(|t| t.e() == &my_entity)
}))?;
```

Higher level helpers can build on this primitive. For example `history_of(entity)` filters
`ancestors(HEAD)` to commits touching a specific entity:

```rust
let changes = ws.checkout(history_of(my_entity))?;
```

## Git Comparison

The table below summarizes Git's revision grammar. Each row links back to the
official documentation. Forms that rely on reflogs or reference objects other
than commits are listed for completeness but are unlikely to be implemented.

| Git Syntax | Planned Equivalent | Reference | Status |
|-----------|-------------------|-----------|--------|
| `A` | `commit(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#_specifying_revisions) | Implemented |
| `A^`/`A^N` | `nth_parent(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revnegHEADv1510) | Not planned |
| `A~N` | `nth_ancestor(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revnegHEADmaster3) | Implemented |
| `A^@` | `parents(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revegHEAD) | Implemented |
| `A^!` | `A minus parents(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revegHEAD-1) | Unimplemented |
| `A^-N` | `A minus nth_parent(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-rev-negHEAD-HEAD-2) | Not planned |
| `A^0` | `commit(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revnegHEADv1510) | Implemented |
| `A^{}` | `deref_tag(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revegv0998) | Unimplemented |
| `A^{type}` | `object_of_type(A, type)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revtypeegv0998commit) | Not planned: non-commit object |
| `A^{/text}` | `search_from(A, text)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revtextegHEADfixnastybug) | Not planned: requires commit message search |
| `:/text` | `search_repo(text)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-textegfixnastybug) | Not planned: requires repository search |
| `A:path` | `blob_at(A, path)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revpathegHEADREADMEmasterREADME) | Not planned: selects a blob not a commit |
| `:[N:]path` | `index_blob(path, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-npatheg0READMEREADME) | Not planned: selects from the index |
| `A..B` | `range(A, B)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-Thetwo-dotRangeNotation) | Implemented |
| `A...B` | `symmetric_diff(A, B)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-Thethree-dotSymmetricDifferenceNotation) | Implemented |
| `^A` | `exclude(reachable(A))` | [gitrevisions](https://git-scm.com/docs/gitrevisions#_commit_exclusions) | Unimplemented |
| `A@{upstream}` | `upstream_of(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-branchnameupstreamegmasterupstreamu) | Not planned: depends on remote config |
| `A@{push}` | `push_target_of(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-branchnamepushegmasterpushpush) | Not planned: depends on remote config |
| `A@{N}` | `reflog(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-refnamenegmaster1) | Not planned: relies on reflog history |
| `A@{<date>}` | `reflog_at(A, date)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-refnamedateegmasteryesterdayHEAD5minutesago) | Not planned: relies on reflog history |
| `@{N}` | `reflog(HEAD, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-neg1) | Not planned: relies on reflog history |
| `@{-N}` | `previous_checkout(N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt--neg-1) | Not planned: relies on reflog history |

Only a subset of Git's revision grammar will likely be supported. Selectors relying on reflog history, remote configuration, or searching commits and blobs add complexity with little benefit for workspace checkout. They are listed above for completeness but remain unplanned for now.

> Note: `range(A, B)` differs subtly from Git's two-dot syntax. It walks parents
> from `B` until a commit from `A` is encountered instead of subtracting the
> entire ancestor closure of `A`. Use `ancestors(A)..B` for Git's behaviour.

## TimeRange

Commits record when they were made via a `timestamp` attribute of type
[`NsTAIInterval`](../src/value/schemas/time.rs). When creating a commit this
interval defaults to `(now, now)` but other tools could provide a wider range
if the clock precision is uncertain. The `TimeRange` selector uses this interval
to gather commits whose timestamps fall between two `Epoch` values:

```rust
use hifitime::Epoch;
use triblespace::repo::time_range;

let since = Epoch::from_unix_seconds(1_609_459_200.0); // 2020-12-01
let now = Epoch::now().unwrap();
let tribles = ws.checkout(time_range(since, now))?;
```

This walks the history from `HEAD` and returns only those commits whose
timestamp interval intersects the inclusive range.

Internally it uses `filter(ancestors(HEAD), ..)` to check each commit's
timestamp range.


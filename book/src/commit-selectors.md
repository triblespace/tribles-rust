# Commit Selectors

The current `Workspace::checkout` API accepts a `CommitSelector` trait which is
implemented for individual handles and standard Rust ranges. While convenient,
this range-based design makes it difficult to compose complex queries over the
commit graph.

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

## Git Comparison

The table below summarizes Git's revision grammar. Each row links back to the
official documentation. Forms that rely on reflogs or reference objects other
than commits are listed for completeness but are unlikely to be implemented.

| Git Syntax | Planned Equivalent | Reference | Status |
|-----------|-------------------|-----------|--------|
| `A` | `commit(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#_specifying_revisions) | Unimplemented |
| `A^`/`A^N` | `nth_parent(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revnegHEADv1510) | Unimplemented |
| `A~N` | `nth_ancestor(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revnegHEADmaster3) | Unimplemented |
| `A^@` | `parents(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revegHEAD) | Unimplemented |
| `A^!` | `A minus parents(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revegHEAD-1) | Unimplemented |
| `A^-N` | `A minus nth_parent(A, N)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-rev-negHEAD-HEAD-2) | Unimplemented |
| `A^0` | `commit(A)` | [gitrevisions](https://git-scm.com/docs/gitrevisions#Documentation/gitrevisions.txt-revnegHEADv1510) | Unimplemented |
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


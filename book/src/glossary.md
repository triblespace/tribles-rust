# Glossary

**Blob**: A chunk of binary data addressed by the hash of its contents. Blobs are
immutable and can live in memory, on disk, or in remote object stores.

**Trible**: A three-part tuple of entity, attribute, and value. Tribles are the
atomic facts that make up all higher level structures in Trible Space.

**Trible Space**: The overall storage model that organises tribles across blobs,
PATCHes, and repositories. It emphasises immutability and content addressing.

**PATCH**: A tree-shaped index used to organise tribles for efficient queries.
Different permutations of the same trible share leaves to resist skewed data.

**Pile**: An append-only collection of blobs such as tribles, patches, and other
data. Piles can be opened from local files or object stores.

**Workspace**: A mutable working area for preparing commits. Once a commit is
finalised, it becomes immutable like the rest of Trible Space.


# PATCH

The **Persistent Adaptive Trie with Cuckoo-compression and Hash-maintenance** (PATCH) is the core data structure used for set operations in Trible Space.
It stores keys in a compressed 256-ary trie where each node uses a byte oriented cuckoo hash table to map to its children.
This single node layout supports anywhere from two to 256 entries, avoiding the complex branching logic of other adaptive tries.

PATCH nodes maintain a rolling hash which allows efficient union, intersection and difference operations over whole subtrees.
Keys can be viewed in different orders with the [`KeyOrdering`](../../src/patch.rs) trait and segmented via [`KeySegmentation`](../../src/patch.rs) to enable prefix based queries.
All updates use copy‑on‑write semantics, so cloning a tree is cheap and safe.

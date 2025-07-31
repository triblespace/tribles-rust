# PATCH

The **Persistent Adaptive Trie with Cuckoo-compression and Hash-maintenance** (PATCH) is the core data structure used for set operations in Trible Space.
It stores keys in a compressed 256-ary trie where each node uses a byte oriented cuckoo hash table to map to its children.
This single node layout supports anywhere from two to 256 entries, avoiding the complex branching logic of other adaptive tries.

Traditional Adaptive Radix Trees (ART) employ multiple node variants like
`Node4`, `Node16` or `Node48` to keep memory usage proportional to the number of
children. PATCH instead compresses every branch with a byte oriented cuckoo hash
table. Each node contains two arrays of candidate slots and inserts may displace
previous entries similar to classic cuckoo hashing. The layout never changes, so
we avoid the branching logic and pointer chasing common in ART implementations
while still achieving high occupancy.

Our byte table uses two hash functions built from a specialised *compressed
permutation*. The first always uses the identity mapping and the second picks a
random bijective byte→byte permutation. The current table size simply masks off
the upper bits to compress these results. Doubling the table reveals one more
significant bit so entries either stay in place or move to bucket `index * 2`.
When all 256 children exist we disable the random permutation and use the
identity for both hashes, turning the full table into a simple array where each
byte already occupies its canonical slot.

The `byte_table_resize_benchmark` demonstrates how densely the table can fill
before triggering a resize. The benchmark inserts all byte values many times
and measures the occupancy that forced each power-of-two table size to grow:

```
ByteTable resize fill - random: 0.863, sequential: 0.972
Per-size fill (random)
  size   2: 1.000  # path compression keeps two-entry nodes fully occupied
  size   4: 0.973
  size   8: 0.899
  size  16: 0.830
  size  32: 0.749
  size  64: 0.735
  size 128: 0.719
  size 256: 1.000  # identity hash maps all 256 children without resizing
Per-size fill (sequential)
  size   2: 1.000  # path compression keeps two-entry nodes fully occupied
  size   4: 1.000
  size   8: 0.993
  size  16: 1.000
  size  32: 0.928
  size  64: 0.925
  size 128: 0.927
  size 256: 1.000  # identity hash maps all 256 children without resizing
```

Random inserts average roughly 86% table fill while sequential inserts hold
about 97% before doubling the table size. Nodes of size two are always 100%
full thanks to path compression, and the final 256‑ary node also reaches 100%
occupancy because of the linear hash, which we now report explicitly instead of
`0.000`. This keeps memory usage predictable without the specialized node
formats used by ART.

PATCH nodes maintain a rolling hash which allows efficient union, intersection and difference operations over whole subtrees.
Keys can be viewed in different orders with the [`KeyOrdering`](../../src/patch.rs) trait and segmented via [`KeySegmentation`](../../src/patch.rs) to enable prefix based queries.
All updates use copy‑on‑write semantics, so cloning a tree is cheap and safe.

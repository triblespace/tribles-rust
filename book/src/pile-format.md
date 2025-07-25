# Pile Format

The on-disk pile keeps every blob and branch in one append-only file. This layout provides a simple **write-ahead log** style database where new data is only appended. It allows both blob and branch storage in a single file while remaining resilient to crashes. The pile file can be memory mapped for fast reads and is safely shared between threads.

While large databases often avoid `mmap` due to pitfalls with partial writes
and page cache thrashing [[1](https://db.cs.cmu.edu/mmap-cidr2022/)], this
design works well for the pile's narrow usage pattern.

## Design Rationale

This format emphasizes **simplicity** over sophisticated on-disk structures.
Appending new blobs rather than rewriting existing data keeps corruption
windows small and avoids complicated page management. Storing everything in a
single file makes a pile easy to back up or replicate over simple transports
while still allowing it to be memory mapped for fast reads. The 64&nbsp;byte
alignment ensures each entry begins on a cache line boundary, which improves
concurrent access patterns and allows safe typed views with the `zerocopy`
crate.

Hash verification only happens when blobs are read. Opening even a very large
pile is therefore fast while still catching corruption before data is used.

Every record begins with a 16&nbsp;byte magic marker that identifies whether it
stores a blob or a branch. The sections below illustrate the layout of each
type.
## Blob Storage
```
                             8 byte  8 byte
            ┌────16 byte───┐┌──────┐┌──────┐┌────────────32 byte───────────┐
          ┌ ┌──────────────┐┌──────┐┌──────┐┌──────────────────────────────┐
 header   │ │magic number A││ time ││length││             hash             │
          └ └──────────────┘└──────┘└──────┘└──────────────────────────────┘
            ┌────────────────────────────64 byte───────────────────────────┐
          ┌ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐
          │ │                                                              │
 payload  │ │              bytes (64byte aligned and padded)               │
          │ │                                                              │
          └ └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┘
```
Each blob entry records its creation timestamp, the length of the payload and
its hash. The payload is padded so the next record begins on a
64&nbsp;byte boundary.

## Branch Storage
```
            ┌────16 byte───┐┌────16 byte───┐┌────────────32 byte───────────┐
          ┌ ┌──────────────┐┌──────────────┐┌──────────────────────────────┐
 header   │ │magic number B││  branch id   ││             hash             │
          └ └──────────────┘└──────────────┘└──────────────────────────────┘
```
Branch entries map a branch identifier to the hash of a blob.
## Recovery
When [`Pile::try_open`] scans an existing file it checks that every header uses a known marker and that the whole record fits. It does not verify any hashes. If a truncated or unknown block is found the function reports the number of bytes that were valid so far using [`OpenError::CorruptPile`].

The convenience wrapper [`Pile::open`] re-runs the same validation and truncates
the file to the valid length if corruption is encountered. This recovers from
interrupted writes by discarding incomplete data.
Hash verification happens lazily only when individual blobs are loaded so that
opening a large pile remains fast.

For more details on interacting with a pile see the [`Pile` struct
documentation](https://docs.rs/tribles/latest/tribles/repo/pile/struct.Pile.html).

[1]: https://db.cs.cmu.edu/mmap-cidr2022/ "The Case Against Memory-Mapped I/O"

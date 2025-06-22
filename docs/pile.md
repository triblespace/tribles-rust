# Pile Format and Recovery

A `Pile` stores blobs and branches sequentially in a single append-only file. Each
record begins with a 16 byte magic marker that identifies whether the block is a
blob or a branch. Blob headers additionally contain a timestamp, the byte length
of the payload and the hash of the blob. Branch headers contain the branch id and
the referenced blob hash.

When opening a file, `Pile::try_open` validates that every block header uses one
of the known markers and that the entire block fits into the file. It does **not**
verify any hashes. If a record is truncated or has an unknown marker, the function
returns `OpenError::CorruptPile { valid_length }` where `valid_length` marks the
number of bytes that belong to well formed blocks.

`Pile::open` provides a convenience wrapper that attempts the same parsing but
truncates the file to `valid_length` whenever such a corruption error is encountered.
This recovers from interrupted writes by discarding incomplete bytes so that the
file can still be used.

Hash verification happens lazily when individual blobs are loaded, keeping the
initial opening cost low.

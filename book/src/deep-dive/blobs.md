# Blobs

Blobs are immutable sequences of bytes used to represent data that does not fit into the fixed 256‑bit value slot of a trible. Each blob is typed by a `BlobSchema` similar to how values use `ValueSchema`. This allows structured data to be serialized into a blob while still tracking its schema.

Values and tribles capture small facts in a fixed width, whereas blobs are used "in the large" for documents, media and other sizable payloads. A blob can therefore represent anything from a single file to a complete archive of tribles.

Converting Rust types to blobs is infallible in practice, so only the `ToBlob` and `TryFromBlob` traits are widely used. The `TryToBlob` and `FromBlob` variants have been dropped to keep the API surface small.

The following example demonstrates creating blobs, archiving a `TribleSet` and signing its contents:

```rust
use tribles::prelude::*;
use tribles::examples::literature;
use tribles::repo;
use valueschemas::{Handle, Blake3};
use blobschemas::{SimpleArchive, LongString};
use rand::rngs::OsRng;
use ed25519_dalek::{Signature, Signer, SigningKey};

// Build a BlobStore and fill it with some data.
let mut memory_store: MemoryBlobStore<Blake3> = MemoryBlobStore::new();

let book_author_id = fucid();
let quote_a: Value<Handle<Blake3, LongString>> = memory_store
    .put("Deep in the human unconscious is a pervasive need for a logical universe that makes sense. But the real universe is always one step beyond logic.")
    .unwrap();
let quote_b = memory_store
    .put("I must not fear. Fear is the mind-killer. Fear is the little-death that brings total obliteration. I will face my fear. I will permit it to pass over me and through me. And when it has gone past I will turn the inner eye to see its path. Where the fear has gone there will be nothing. Only I will remain.")
    .unwrap();

let set = entity!{
   literature::title: "Dune",
   literature::author: &book_author_id,
   literature::quote: quote_a,
   literature::quote: quote_b
};

// Serialize the TribleSet and store it as another blob.
let archived_set_handle: Value<Handle<Blake3, SimpleArchive>> = memory_store.put(&set).unwrap();

let mut csprng = OsRng;
let commit_author_key: SigningKey = SigningKey::generate(&mut csprng);
let signature: Signature = commit_author_key.sign(
    &memory_store
        .reader()
        .unwrap()
        .get::<Blob<SimpleArchive>, SimpleArchive>(archived_set_handle)
        .unwrap()
        .bytes,
);

// Store the handle in another TribleSet.
let _meta_set = entity!{
   repo::content: archived_set_handle,
   repo::short_message: "Initial commit",
   repo::signed_by: commit_author_key.verifying_key(),
   repo::signature_r: signature,
   repo::signature_s: signature,
};
```

Blobs complement tribles and values by handling large payloads while keeping the
core data structures compact. A blob's hash, computed via a chosen
`HashProtocol`, acts as a stable handle that can be embedded into tribles or
other blobs, enabling content‑addressed references without copying the payload.

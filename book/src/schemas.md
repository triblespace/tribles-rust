# Schemas

Trible Space stores data in strongly typed values and blobs. A *schema* defines
the language‑agnostic byte layout for these types: [`Value`]s always occupy
exactly 32&nbsp;bytes while [`Blob`]s may be any length. Schemas translate those
raw bytes to concrete application types and decouple persisted data from a
particular implementation. This separation lets you refactor to new libraries
or frameworks without rewriting what's already stored. The crate ships with a
collection of ready‑made schemas located in
[`tribles::value::schemas`](https://docs.rs/tribles/latest/tribles/value/schemas/index.html) and
[`tribles::blob::schemas`](https://docs.rs/tribles/latest/tribles/blob/schemas/index.html).

### Why 32 bytes?

Storing arbitrary Rust types requires a portable representation. Instead of
human‑readable identifiers like RDF's URIs, Tribles uses a fixed 32‑byte array
for all values. This size provides enough entropy to embed intrinsic
identifiers—typically cryptographic hashes—when a value references data stored
elsewhere in a blob. Keeping the width constant avoids platform‑specific
encoding concerns and makes it easy to reason about memory usage.

### Conversion traits

Schemas define how to convert between raw bytes and concrete Rust types. The
conversion traits `ToValue`/`FromValue` and their fallible counterparts live on
the schema types rather than on `Value` itself, avoiding orphan‑rule issues when
supporting external data types. The `Value` wrapper treats its bytes as opaque;
schemas may validate them or reject invalid patterns during conversion.

### Schema identifiers

Every schema declares a unique 128‑bit identifier such as `VALUE_SCHEMA_ID`
(and optionally `BLOB_SCHEMA_ID` for blob handles). Persisting these IDs allows
applications to look up the appropriate schema at runtime, even when they were
built against different code. The `schema_id` method on `Value` and `Blob`
returns the identifier so callers can dispatch to the correct conversion logic.

## Built‑in value schemas

The crate provides the following value schemas out of the box:
- `GenId` &ndash; an abstract 128 bit identifier.
- `ShortString` &ndash; a UTF-8 string up to 32 bytes.
- `U256BE` / `U256LE` &ndash; 256-bit unsigned integers.
- `I256BE` / `I256LE` &ndash; 256-bit signed integers.
- `R256BE` / `R256LE` &ndash; 256-bit rational numbers.
- `F256BE` / `F256LE` &ndash; 256-bit floating point numbers.
- `Hash` and `Handle` &ndash; cryptographic digests and blob handles (see [`hash.rs`](../src/value/schemas/hash.rs)).
- `ED25519RComponent`, `ED25519SComponent` and `ED25519PublicKey` &ndash; signature fields and keys.
- `NsTAIInterval` to encode time intervals.
- `UnknownValue` as a fallback when no specific schema is known.

```rust
use tribles::value::schemas::shortstring::ShortString;
use tribles::value::{ToValue, ValueSchema};

let v = "hi".to_value::<ShortString>();
assert_eq!(v.schema_id(), ShortString::VALUE_SCHEMA_ID);
```

## Built‑in blob schemas

The crate also ships with these blob schemas:

- `LongString` for arbitrarily long UTF‑8 strings.
- `SimpleArchive` which stores a raw sequence of tribles.
- `SuccinctArchive` providing a compressed index for offline queries.
- `UnknownBlob` for data of unknown type.

```rust
use tribles::blob::schemas::longstring::LongString;
use tribles::blob::{ToBlob, BlobSchema};

let b = "example".to_blob::<LongString>();
assert_eq!(LongString::BLOB_SCHEMA_ID, b.schema_id());
```

## Defining new schemas

Custom formats implement [`ValueSchema`] or [`BlobSchema`].  A unique identifier
serves as the schema ID.  The example below defines a little-endian `u64` value
schema and a simple blob schema for arbitrary bytes.

```rust
{{#include ../../examples/custom_schema.rs:custom_schema}}
```

See [`examples/custom_schema.rs`](https://github.com/triblespace/tribles-rust/blob/main/examples/custom_schema.rs) for the full
source.

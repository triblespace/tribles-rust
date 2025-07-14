# Schemas

Trible Space stores data in strongly typed values and blobs.  A *schema*
describes a language‑agnostic data type with a specific byte representation:
exactly 32&nbsp;bytes for a [`Value`] and an arbitrary number of bytes for a
[`Blob`].  These abstract types can be converted to the concrete types of your
application but decouple stored data from any particular implementation.  This
also means you can refactor to new libraries or frameworks without rewriting
what's already stored. The crate ships with a collection of ready‑made schemas located in
[`src/value/schemas`](../src/value/schemas) and
[`src/blob/schemas`](../src/blob/schemas).

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
{{#include ../examples/custom_schema.rs:beginning:ending}}
```

See [`examples/custom_schema.rs`](../examples/custom_schema.rs) for the full
source.

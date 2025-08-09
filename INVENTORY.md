# Inventory

## Potential Removals
- None at the moment.

## Desired Functionality
- Provide additional examples showcasing advanced queries and repository usage.
- Helper to derive delta `TribleSet`s for `pattern_changes!` so callers don't
  have to compute them manually.
- Explore replacing `CommitSelector` ranges with a set-based API
  built on commit reachability. The goal is to mirror git's revision
  selection semantics (similar to `rev-list` or `rev-parse`).
  Combinators like `union`, `intersection` and `difference` should let
  callers express queries such as "A minus B" or "ancestors of A
  intersect B". Commit sets themselves would be formed by primitives
  like `ancestors(<commit>)` and `descendants(<commit>)` so selectors
  map directly to the commit graph.
- Generate namespaces from a `TribleSet` description so tooling can
  derive them programmatically. Rewriting `pattern!` as a procedural
  macro will be the first step toward this automation.
- Benchmark PATCH performance across typical workloads.
- Investigate the theoretical complexity of PATCH operations.
- Measure practical space usage for PATCH with varying dataset sizes.
- Benchmark recursive `ByteTable` displacement planner versus the greedy random insert to measure fill rate and performance across intermediate table sizes.
- Explore converting the recursive `ByteTable` planner into an iterative search to reduce stack usage.
- Implement a garbage collection mechanism that scans branch and commit
  archives without fully deserialising them to find reachable blob handles.
  Anything not discovered this way can be forgotten by the underlying store.
- Generalise the declarative key description utilities to other key types so
  segment layouts and orderings can be defined once and generated automatically.
- Provide a macro to declare key layouts that emits segmentation and
  ordering implementations for PATCH at compile time.
- Expose segment iterators on PATCH using `KeyOrdering`'s segment permutation instead of raw key ranges.

## Additional Built-in Schemas
The existing collection of schemas covers the basics like strings, large
integers and archives.  The following ideas could broaden what can be stored
without custom extensions:

### Value schemas
- `Uuid` for RFC&nbsp;4122 identifiers.
- `Ipv4Addr` and `Ipv6Addr` to store network addresses.  IPv6 could dedicate
  spare bits to a port or service code.
- `SocketAddr` representing an IP address and port in one value.
- `MacAddr` for layer‑2 hardware addresses.
- `Duration` for relative time spans.
- `GeoPoint` with latitude and longitude stored as two 64‑bit floats.
- `RgbaColor` packing four 8‑bit channels into one value.
- `BigDecimal` for high‑precision numbers up to 256 bits.

### Blob schemas
- `Json`, `Cbor` and `Yaml` for structured data interchange.
- `Csv` for comma‑separated tables.
- `Protobuf` or `MessagePack` for compact typed messages.
- `Parquet` and `Arrow` for columnar analytics workloads.
- `Lance` for memory-mapped columnar datasets.
- `CompressedBlob` wrapping arbitrary content with deflate or zip compression.
- `WasmModule` for executable WebAssembly.
- `OnnxModel` or `Safetensors` for neural networks.
- `HnswIndex` for vector search structures.
- `TantivyIndex` capturing a full-text search corpus.
- `Url` for web links and other IRIs; best stored as a blob due to the value
  size limit.
- `Html` or `Xml` for markup documents.
- `Markdown` for portable text.
- `Svg` for vector graphics.
- `Png` and `Jpeg` images.
- `Pdf` for print‑ready documents.

Formats with solid memory-mapping support in the Rust ecosystem should be
prioritized for efficient zero-copy access.

## Documentation
- Move the "Portability & Common Formats" overview from `src/value.rs` into a
  dedicated chapter of the book.
- Migrate the blob module introduction in `src/blob.rs` so the crate docs focus
  on API details.
- Extract the repository design discussion and Git parallels from `src/repo.rs`
  into the book.
- Split out the lengthy explanation of trible structure from `src/trible.rs`
  and consolidate it with the deep dive chapter.
- Add a FAQ chapter to the book summarising common questions.

## Discovered Issues
- No open issues recorded yet.
- Enforce `PREFIX_LEN` never exceeds `KEY_LEN` when checking prefixes.

# Introduction

Welcome to the **Tribles Book**. This guide provides a gentle introduction to
Trible Space, its design goals and how to use the `tribles` crate.  The aim is to
present a clear narrative for newcomers while linking to in‑depth reference
material for those who want to dig deeper.

Trible Space combines ideas from databases and version control. Data is stored
in small immutable blobs that can live in memory, on disk or inside remote
object stores without conversion. Cryptographic identifiers ensure integrity and
enable efficient sharing across systems.

What makes Trible Space stand out?

- **Content‑addressed immutability** – values are stored by the hash of their
  contents, making them easy to verify and share.
- **Lightweight queries** – work over unordered collections like hash maps,
  trible sets or custom full‑text/semantic indexes; you can embed
  domain‑specific DSLs and they run fast enough to use like everyday language
  constructs.
- **Repository workflows** – history and collaboration follow familiar
  version‑control patterns.

The opening chapters introduce these ideas at a gentle pace. Later sections dive
into architecture, query semantics, schemas and commit selectors so you can
build and reason about complex data sets.

While this book walks you through the basics, the crate documentation offers a
complete API reference. Use the links throughout the text to jump directly to
the modules that interest you.

If you would like to work on Tribles yourself, check out [Developing Locally](contributing.md).

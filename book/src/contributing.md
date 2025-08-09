# Developing Locally

To build and test Tribles yourself you need a recent Rust toolchain. Install it from [rustup.rs](https://rustup.rs/).

The repository provides helper scripts for common workflows:

- `./scripts/preflight.sh` formats the code, runs the full test suite and rebuilds this book. Run it before committing.
- `./scripts/devtest.sh` executes only the tests for faster feedback during development.
- `./scripts/build_book.sh` rebuilds the documentation once [`mdbook`](https://rust-lang.github.io/mdBook/) is installed with `cargo install mdbook`.

These scripts keep the project formatted, tested and documented while you iterate locally.

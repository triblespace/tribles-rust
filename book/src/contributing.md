# Developing Locally

To build and test Tribles yourself you need a recent Rust toolchain. Install it from [rustup.rs](https://rustup.rs/).

Run `./scripts/preflight.sh` from the repository root to format the code and execute the full test suite. For quick iterations `./scripts/devtest.sh` runs only the tests.

If you want to render this book locally first install `mdbook` with `cargo install mdbook` and then run `./scripts/build_book.sh`.

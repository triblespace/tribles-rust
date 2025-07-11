#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Ensure rustfmt is installed
rustup component add rustfmt

# Run formatting check and tests
cargo fmt -- --check
cargo test

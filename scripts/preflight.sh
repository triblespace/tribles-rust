#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Ensure rustfmt is installed
cargo install rustfmt --locked --quiet || true

# Run formatting check and tests
cargo fmt -- --check
cargo test

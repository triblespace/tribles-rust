#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Ensure required tools are available
if ! command -v rustfmt >/dev/null 2>&1; then
  echo "rustfmt not found. Installing via rustup..."
  rustup component add rustfmt
fi

# Run formatting check and tests
cargo fmt -- --check
cargo test

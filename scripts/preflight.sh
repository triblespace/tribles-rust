#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Run formatting check, tests, and Kani verification
# Assumes rustfmt and the Kani verifier are already installed.
cargo fmt -- --check
cargo test
cargo kani --workspace

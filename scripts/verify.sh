#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Ensure rustfmt and cargo-kani are installed
cargo install rustfmt --locked --quiet || true
cargo install kani-verifier --locked --quiet || true

# Run all Kani proofs in the workspace. Additional stress or property tests can
# be added here as needed.
cargo kani --workspace

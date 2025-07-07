#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Ensure Kani verifier is available
if ! cargo kani --version >/dev/null 2>&1; then
  echo "cargo-kani not found. Installing Kani verifier..."
  cargo install --locked kani-verifier
fi

# Run all Kani proofs in the workspace. Additional stress or property tests can
# be added here as needed.
cargo kani --workspace

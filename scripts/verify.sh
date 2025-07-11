#!/usr/bin/env bash
set -euo pipefail

# Move to repository root
cd "$(dirname "$0")/.."

# Run all Kani proofs in the workspace. Additional stress or property tests can
# be added here as needed.
cargo kani --workspace

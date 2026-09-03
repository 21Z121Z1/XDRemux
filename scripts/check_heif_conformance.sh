#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The Rust example performs post-write portable structural validation before
# its output is checked against the Rust structural contract.
cargo build --locked -q -p xdremux-heif --example heif_conformance
cargo test --locked -p xdremux-heif

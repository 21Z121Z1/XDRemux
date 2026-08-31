#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The Rust example performs post-write portable structural validation before
# its output is compared byte-for-byte with the current Swift writer corpus.
cargo build --locked -q -p xdremux-heif --example heif_conformance
swift test --filter HEIFRustConformanceTests/testSwiftAndRustDirectHEVCWriterAreByteExact

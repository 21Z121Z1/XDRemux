#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --locked -q -p xdremux-heif --example heif_conformance
swift test --filter HEIFRustConformanceTests/testSwiftAndRustDirectHEVCWriterAreByteExact

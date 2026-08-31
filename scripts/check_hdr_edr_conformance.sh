#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-hdr-edr.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

SWIFT_OUT="$TMP/swift-edr.txt"
RUST_OUT="$TMP/rust-edr.txt"

XDREMUX_HDR_ORACLE_OUTPUT="$SWIFT_OUT" \
  swift test --filter HDRRustConformanceOracleTests/testEmitEDRVectorsForRustDifferential >/dev/null

cargo run --quiet --locked -p xdremux-hdr --bin xdremux-hdr-vectors -- \
  Tests/fixtures/hdr_edr_cases.tsv > "$RUST_OUT"

if ! diff -u "$SWIFT_OUT" "$RUST_OUT"; then
  echo "Swift/Rust EDR conformance failed" >&2
  exit 1
fi

RESOLVE_COUNT="$(grep -c '^resolve' "$SWIFT_OUT")"
KNEE_COUNT="$(grep -c '^knee' "$SWIFT_OUT")"
if [[ "$RESOLVE_COUNT" -ne 17 || "$KNEE_COUNT" -ne 8 ]]; then
  echo "unexpected EDR vector coverage: resolve=$RESOLVE_COUNT knee=$KNEE_COUNT" >&2
  exit 1
fi

echo "PASS Swift/Rust EDR vectors: resolve=$RESOLVE_COUNT knee=$KNEE_COUNT"

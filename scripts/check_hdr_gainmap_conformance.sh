#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-hdr-gainmap.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

SWIFT_OUT="$TMP/swift-gainmap.txt"
RUST_OUT="$TMP/rust-gainmap.txt"

XDREMUX_GAINMAP_ORACLE_OUTPUT="$SWIFT_OUT" \
  swift test --filter HDRRustConformanceOracleTests/testEmitGainMapVectorsForRustDifferential >/dev/null

cargo run --quiet --locked -p xdremux-hdr --bin xdremux-gainmap-vectors -- \
  Tests/fixtures/hdr_edr_cases.tsv > "$RUST_OUT"

if ! diff -u "$SWIFT_OUT" "$RUST_OUT"; then
  echo "Swift/Rust gain map conformance failed" >&2
  exit 1
fi

CASE_COUNT="$(grep -c '^gainmap' "$SWIFT_OUT")"
if [[ "$CASE_COUNT" -ne 3 ]]; then
  echo "unexpected gain map vector coverage: cases=$CASE_COUNT" >&2
  exit 1
fi

echo "PASS Swift/Rust gain map vectors: cases=$CASE_COUNT exhaustive-byte-domain=512"

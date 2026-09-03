#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-hdr-edr.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

RUST_OUT="$TMP/rust-edr.txt"

cargo run --quiet --locked -p xdremux-hdr --bin xdremux-hdr-vectors -- \
  Tests/fixtures/hdr_edr_cases.tsv > "$RUST_OUT"

RESOLVE_COUNT="$(grep -c '^resolve' "$RUST_OUT")"
KNEE_COUNT="$(grep -c '^knee' "$RUST_OUT")"
if [[ "$RESOLVE_COUNT" -ne 17 || "$KNEE_COUNT" -ne 8 ]]; then
  echo "unexpected EDR vector coverage: resolve=$RESOLVE_COUNT knee=$KNEE_COUNT" >&2
  exit 1
fi

echo "PASS Rust EDR vectors: resolve=$RESOLVE_COUNT knee=$KNEE_COUNT"

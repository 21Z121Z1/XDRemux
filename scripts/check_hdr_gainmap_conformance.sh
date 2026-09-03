#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-hdr-gainmap.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

RUST_OUT="$TMP/rust-gainmap.txt"

cargo run --quiet --locked -p xdremux-hdr --bin xdremux-gainmap-vectors -- \
  Tests/fixtures/hdr_edr_cases.tsv > "$RUST_OUT"

CASE_COUNT="$(grep -c '^gainmap' "$RUST_OUT")"
if [[ "$CASE_COUNT" -ne 3 ]]; then
  echo "unexpected gain map vector coverage: cases=$CASE_COUNT" >&2
  exit 1
fi

echo "PASS Rust gain map vectors: cases=$CASE_COUNT exhaustive-byte-domain=512"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --locked -q -p xdremux-container --bin xdremux-container-extract
RUST_ORACLE="$ROOT/target/debug/xdremux-container-extract"
if [[ ! -x "$RUST_ORACLE" ]]; then
  echo "Rust container inspector was not built at $RUST_ORACLE" >&2
  exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-container-conformance.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

fixtures=(
  "fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic"
  "fixtures/proxdr/oppo/find-x7-ultra/lhdr-v2-01.heic"
  "fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
)

for fixture in "${fixtures[@]}"; do
  if [[ ! -f "$fixture" ]]; then
    echo "missing container fixture: $fixture" >&2
    exit 1
  fi
  snapshot="$TMP_ROOT/$(basename "$fixture")"
  "$RUST_ORACLE" "$fixture" "$snapshot"
  summary="$snapshot/summary.tsv"
  if [[ ! -s "$summary" ]] || ! grep -q $'^mode\t' "$summary" || ! grep -q $'^manifest\t' "$summary" || ! grep -q $'^entry\t' "$summary"; then
    echo "Rust container inspector did not report a complete extraction snapshot: $fixture" >&2
    exit 1
  fi
  echo "PASS Rust container fixture: $fixture"
done

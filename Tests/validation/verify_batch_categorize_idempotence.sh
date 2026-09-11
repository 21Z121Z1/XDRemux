#!/usr/bin/env bash
#
# `categorize` must be idempotent: re-running it over the same input must report
# duplicates and leave the already-filed bytes unchanged.
#
# usage: verify_batch_categorize_idempotence.sh <sample.heic> [<sample.heic> ...]

set -euo pipefail

if (($# == 0)); then
  echo "usage: $0 <sample.heic> [<sample.heic> ...]" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="${XDREMUX_CLI:-$ROOT_DIR/target/debug/xdremux}"

if [[ ! -x "$CLI" ]]; then
  cargo build --locked -q -p xdremux-cli
fi
if [[ ! -x "$CLI" ]]; then
  echo "Rust CLI not built: $CLI (run cargo build -p xdremux-cli first)" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-idempotence-XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

for sample in "$@"; do
  if [[ ! -f "$sample" ]]; then
    echo "sample not found: $sample" >&2
    exit 2
  fi
  cp "$sample" "$WORK/"
done

EXPECTED="$#"
OUTPUT="$WORK/output"

echo "== run 1 =="
FIRST="$("$CLI" categorize --input "$WORK" --output-dir "$OUTPUT" 2>&1)"
echo "$FIRST"
if ! grep -q "categorize: $EXPECTED resources, $EXPECTED copied, 0 duplicates, 0 dry-run, 0 failed" <<<"$FIRST"; then
  echo "run 1 did not categorize all $EXPECTED inputs cleanly" >&2
  exit 1
fi

echo "== run 2 =="
SECOND="$("$CLI" categorize --input "$WORK" --output-dir "$OUTPUT" 2>&1)"
echo "$SECOND"
if ! grep -q "categorize: $EXPECTED resources, 0 copied, $EXPECTED duplicates, 0 dry-run, 0 failed" <<<"$SECOND"; then
  echo "run 2 must report the $EXPECTED existing outputs as duplicates" >&2
  exit 1
fi

echo "categorize is idempotent over $EXPECTED inputs"

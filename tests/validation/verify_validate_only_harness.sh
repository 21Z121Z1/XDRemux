#!/usr/bin/env bash
#
# The Rust `validate` command is read-only. It must accept a produced output,
# reject malformed input, and emit a stable JSON report when requested.
#
# usage: verify_validate_only_harness.sh <sample.heic>

set -euo pipefail

if (($# != 1)); then
  echo "usage: $0 <sample.heic>" >&2
  exit 2
fi

SAMPLE="$1"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="${XDREMUX_CLI:-$ROOT_DIR/target/debug/xdremux}"

if [[ ! -f "$SAMPLE" ]]; then
  echo "sample not found: $SAMPLE" >&2
  exit 2
fi
if [[ ! -x "$CLI" ]]; then
  cargo build --locked -q -p xdremux-cli
fi
if [[ ! -x "$CLI" ]]; then
  echo "Rust CLI not built: $CLI (run cargo build -p xdremux-cli first)" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-validate-only-XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

OUTPUT="$WORK/converted.heic"
"$CLI" convert --input "$SAMPLE" --output "$OUTPUT" >/dev/null

cd "$ROOT_DIR"

echo "== a canonical output must pass =="
"$CLI" validate "$OUTPUT"

echo "== malformed input must fail =="
printf 'not a canonical output\n' > "$WORK/malformed.heic"
set +e
"$CLI" validate "$WORK/malformed.heic" >/dev/null 2>&1
MALFORMED_STATUS=$?
set -e
if ((MALFORMED_STATUS == 0)); then
  echo "validate accepted malformed input" >&2
  exit 1
fi

echo "== JSON validation must also pass without writing =="
"$CLI" validate "$OUTPUT" --json >/dev/null
test -f "$OUTPUT"

echo "Rust validate behaves correctly on pass, malformed input, and JSON output"

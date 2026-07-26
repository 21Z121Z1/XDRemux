#!/usr/bin/env bash
#
# ci.yml validates already-produced fixture outputs with
# `verify_swift_cli_sample.py --validate-only`. That mode must assert the
# gain-map pixel format of an existing file without converting it, and it must
# actually fail on a mismatch — a validator that always passes is worse than
# none.
#
# usage: verify_validate_only_harness.sh <sample.heic>

set -euo pipefail

if (($# != 1)); then
  echo "usage: $0 <sample.heic>" >&2
  exit 2
fi

SAMPLE="$1"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="$ROOT_DIR/.build/debug/xdremux"
HARNESS="$ROOT_DIR/Tests/validation/verify_swift_cli_sample.py"

if [[ ! -f "$SAMPLE" ]]; then
  echo "sample not found: $SAMPLE" >&2
  exit 2
fi
if [[ ! -x "$CLI" ]]; then
  echo "Swift CLI not built: $CLI (run swift build first)" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-validate-only-XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

OUTPUT="$WORK/converted.heic"
"$CLI" convert --input "$SAMPLE" --output "$OUTPUT" >/dev/null

cd "$ROOT_DIR"

echo "== matching pixel format must pass =="
python3 "$HARNESS" --validate-only --input "$OUTPUT" --expected-pixel-format L008

echo "== mismatching pixel format must fail =="
set +e
python3 "$HARNESS" --validate-only --input "$OUTPUT" --expected-pixel-format 420v >/dev/null 2>&1
MISMATCH_STATUS=$?
set -e
if ((MISMATCH_STATUS == 0)); then
  echo "--validate-only accepted the wrong pixel format" >&2
  exit 1
fi

echo "== --validate-only must reject conversion-only flags =="
set +e
python3 "$HARNESS" --validate-only --in-place --input "$OUTPUT" \
  --expected-pixel-format L008 >/dev/null 2>&1
COMBINED_STATUS=$?
set -e
if ((COMBINED_STATUS == 0)); then
  echo "--validate-only accepted --in-place, which performs a conversion" >&2
  exit 1
fi

echo "--validate-only behaves correctly on pass, mismatch, and misuse"

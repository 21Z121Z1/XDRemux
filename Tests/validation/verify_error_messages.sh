#!/usr/bin/env bash
#
# Error text and help text are product surfaces, so exercise them through the
# real binary on real files rather than trusting the unit-level string checks.
#
# usage: verify_error_messages.sh <proxdr-sample.heic>

set -euo pipefail

if (($# != 1)); then
  echo "usage: $0 <proxdr-sample.heic>" >&2
  exit 2
fi

SAMPLE="$1"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="$ROOT_DIR/.build/debug/xdremux"

if [[ ! -f "$SAMPLE" ]]; then
  echo "sample not found: $SAMPLE" >&2
  exit 2
fi
if [[ ! -x "$CLI" ]]; then
  echo "Swift CLI not built: $CLI (run swift build first)" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-error-text-XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

expect_contains() {
  local haystack="$1" needle="$2" label="$3"
  if ! grep -qF -- "$needle" <<<"$haystack"; then
    echo "$label: expected to find \"$needle\" in:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

expect_absent() {
  local haystack="$1" needle="$2" label="$3"
  if grep -qF -- "$needle" <<<"$haystack"; then
    echo "$label: did not expect \"$needle\" in:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

echo "== help text names the real binary and documents every option =="
HELP="$("$CLI" --help 2>&1)"
expect_contains "$HELP" "xdremux convert" "help"
expect_absent "$HELP" "XDRemux.swift" "help"
for option in --family --input-processing --tmap-format --oppo-camera-tail --oppo-compat \
              --glob --jobs --checkpoint --categorize --dry-run --debug-dir; do
  expect_contains "$HELP" "$option" "help"
done

echo "== converting an already-converted file explains there is nothing to do =="
CONVERTED="$WORK/converted.heic"
"$CLI" convert --input "$SAMPLE" --output "$CONVERTED" >/dev/null
set +e
AGAIN="$("$CLI" convert --input "$CONVERTED" --output "$WORK/again.heic" 2>&1)"
AGAIN_STATUS=$?
set -e
if ((AGAIN_STATUS == 0)); then
  echo "re-converting an output should fail rather than silently rewrite it" >&2
  exit 1
fi
expect_contains "$AGAIN" "already converted" "re-convert"
expect_contains "$AGAIN" "ISO 21496-1" "re-convert"
expect_absent "$AGAIN" "local.hdr.meta.data" "re-convert"

echo "== converting an unrelated file says it is not a ProXDR photo =="
printf 'not an image at all' >"$WORK/bogus.heic"
set +e
BOGUS="$("$CLI" convert --input "$WORK/bogus.heic" --output "$WORK/bogus-out.heic" 2>&1)"
BOGUS_STATUS=$?
set -e
if ((BOGUS_STATUS == 0)); then
  echo "converting a non-photo should fail" >&2
  exit 1
fi
expect_contains "$BOGUS" "not a ProXDR photo" "unsupported input"
expect_absent "$BOGUS" "local.hdr.meta.data" "unsupported input"

echo "== a batch failure stays one readable line =="
mkdir -p "$WORK/batch"
cp "$SAMPLE" "$WORK/batch/good.heic"
printf 'not an image at all' >"$WORK/batch/bad.heic"
set +e
BATCH="$("$CLI" batch --input-dir "$WORK/batch" --output-dir "$WORK/batch-out" 2>&1)"
set -e
expect_contains "$BATCH" "failed bad.heic: not a ProXDR photo" "batch"
expect_contains "$BATCH" "batch complete: 1 converted, 0 skipped, 1 failed" "batch"
expect_contains "$BATCH" "run the same command again" "batch"
FAILURE_LINE="$(grep "^failed bad.heic" <<<"$BATCH" | head -1)"
if ((${#FAILURE_LINE} > 120)); then
  echo "batch failure line is ${#FAILURE_LINE} chars; list output must stay terse:" >&2
  echo "$FAILURE_LINE" >&2
  exit 1
fi

echo "error and help text behave as documented"

#!/usr/bin/env bash
#
# Error text and help text are product surfaces, so exercise them through the
# real Rust binary on a real file rather than trusting unit-level string checks.
#
# usage: verify_error_messages.sh <proxdr-sample.heic>

set -euo pipefail

if (($# != 1)); then
  echo "usage: $0 <proxdr-sample.heic>" >&2
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

expect_option_absent() {
  local haystack="$1" option="$2" label="$3"
  if grep -Eq -- "(^|[[:space:]])${option}([[:space:]]|$)" <<<"$haystack"; then
    echo "$label: did not expect option $option in:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

echo "== generated help owns the command tree and public options =="
ROOT_HELP="$("$CLI" --help 2>&1)"
expect_absent "$ROOT_HELP" "Swift" "root help"
for command in inspect convert batch categorize validate; do
  expect_contains "$ROOT_HELP" "$command" "root help"
done

CONVERT_HELP="$("$CLI" convert --help 2>&1)"
for option in --input --output --oppo-compatible --apple-styles --apple-portrait; do
  expect_contains "$CONVERT_HELP" "$option" "convert help"
done
for option in --family --input-processing --tmap-format --oppo-camera-tail --oppo-compat --debug-dir; do
  expect_option_absent "$CONVERT_HELP" "$option" "convert help"
done

BATCH_HELP="$("$CLI" batch --help 2>&1)"
for option in --input-dir --output-dir --jobs --checkpoint --categorize \
              --resume --skip-existing --oppo-compatible --apple-styles --apple-portrait; do
  expect_contains "$BATCH_HELP" "$option" "batch help"
done

echo "== validating a produced output is a separate read-only operation =="
CONVERTED="$WORK/converted.heic"
"$CLI" convert --input "$SAMPLE" --output "$CONVERTED" >/dev/null
"$CLI" validate "$CONVERTED" >/dev/null

echo "== converting an unrelated file reports a direct input error =="
printf 'not an image at all' >"$WORK/bogus.heic"
set +e
BOGUS="$("$CLI" convert --input "$WORK/bogus.heic" --output "$WORK/bogus-out.heic" 2>&1)"
BOGUS_STATUS=$?
set -e
if ((BOGUS_STATUS == 0)); then
  echo "converting a non-photo should fail" >&2
  exit 1
fi
expect_contains "$BOGUS" "unsupported input" "unsupported input"
expect_absent "$BOGUS" "Swift" "unsupported input"

echo "== a batch failure stays one readable line =="
mkdir -p "$WORK/batch"
cp "$SAMPLE" "$WORK/batch/good.heic"
printf 'not an image at all' >"$WORK/batch/bad.heic"
set +e
BATCH="$("$CLI" batch --input-dir "$WORK/batch" --output-dir "$WORK/batch-out" 2>&1)"
set -e
expect_contains "$BATCH" "failed: bad.heic" "batch"
expect_contains "$BATCH" "batch: 2 processed, 1 succeeded, 1 failed" "batch"
FAILURE_LINE="$(grep "failed: bad.heic" <<<"$BATCH" | head -1)"
if ((${#FAILURE_LINE} > 160)); then
  echo "batch failure line is ${#FAILURE_LINE} chars; list output must stay terse:" >&2
  echo "$FAILURE_LINE" >&2
  exit 1
fi

echo "Rust error and help text behave as documented"

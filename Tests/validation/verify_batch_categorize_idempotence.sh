#!/usr/bin/env bash
#
# `batch --categorize` must be idempotent: re-running it over a directory it
# already filed must skip the existing outputs, not re-enumerate the
# capture-mode folders it wrote and fail every file in them.
#
# usage: verify_batch_categorize_idempotence.sh <sample.heic> [<sample.heic> ...]

set -euo pipefail

if (($# == 0)); then
  echo "usage: $0 <sample.heic> [<sample.heic> ...]" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="$ROOT_DIR/.build/debug/xdremux"

if [[ ! -x "$CLI" ]]; then
  echo "Swift CLI not built: $CLI (run swift build first)" >&2
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

echo "== run 1 =="
FIRST="$("$CLI" batch --input-dir "$WORK" --output-dir "$WORK" --categorize 2>&1)"
echo "$FIRST"
if ! grep -q "converted $EXPECTED files, skipped-existing 0 files, failed 0 files" <<<"$FIRST"; then
  echo "run 1 did not convert all $EXPECTED inputs cleanly" >&2
  exit 1
fi

echo "== run 2 =="
set +e
SECOND="$("$CLI" batch --input-dir "$WORK" --output-dir "$WORK" --categorize 2>&1)"
SECOND_STATUS=$?
set -e
echo "$SECOND"

if ((SECOND_STATUS != 0)); then
  echo "run 2 exited $SECOND_STATUS; a repeated categorized batch must succeed" >&2
  exit 1
fi
if ! grep -q "converted 0 files, skipped-existing $EXPECTED files, failed 0 files" <<<"$SECOND"; then
  echo "run 2 must skip the $EXPECTED existing outputs and fail nothing" >&2
  exit 1
fi
if grep -q "failed to locate plausible 144-byte" <<<"$SECOND"; then
  echo "run 2 re-enumerated its own categorized output" >&2
  exit 1
fi

echo "batch --categorize is idempotent over $EXPECTED inputs"

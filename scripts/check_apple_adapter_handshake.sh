#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Apple adapter handshake requires macOS" >&2
  exit 2
fi

swift build --product xdremux-apple-adapter
ADAPTER="$(swift build --show-bin-path)/xdremux-apple-adapter"
test -x "$ADAPTER"

RESPONSE="$(printf '%s\n' '{"schema_version":1,"operation":"capabilities"}' | "$ADAPTER")"
python3 - "$RESPONSE" <<'PY'
import json
import sys

response = json.loads(sys.argv[1])
if response.get("schema_version") != 1:
    raise SystemExit(f"unexpected Apple adapter schema: {response!r}")
capabilities = response.get("capabilities")
if not isinstance(capabilities, list):
    raise SystemExit(f"Apple adapter capabilities must be a list: {response!r}")
if sorted(capabilities) != ["photographic-styles", "portrait"]:
    raise SystemExit(f"unexpected Apple adapter capabilities: {capabilities!r}")
PY

TEST_INPUT="$PWD/fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"
test -f "$TEST_INPUT"

# This gate proves the committed Rust source can actually compose with the
# Swift adapter. Use a fresh target directory so a restored Cargo cache cannot
# make an uncompiled or previously different runtime source appear green.
TEST_TARGET="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-apple-adapter.XXXXXX")"
trap 'rm -rf "$TEST_TARGET"' EXIT
CARGO_TARGET_DIR="$TEST_TARGET" \
XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE="$ADAPTER" \
XDREMUX_APPLE_ADAPTER_TEST_INPUT="$TEST_INPUT" \
  cargo test --locked -p xdremux-runtime \
    --test apple_adapter \
    --test apple_portrait_rend

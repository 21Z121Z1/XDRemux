#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Apple adapter handshake requires macOS" >&2
  exit 2
fi

swift build --target XDRemuxAppleAdapter
ADAPTER="$(swift build --show-bin-path)/XDRemuxAppleAdapter"
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

XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE="$ADAPTER" \
  cargo test --locked -p xdremux-runtime --test apple_adapter

#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Apple adapter handshake requires macOS" >&2
  exit 2
fi

swift build --product xdremux-apple-adapter
ADAPTER="$(swift build --show-bin-path)/xdremux-apple-adapter"
test -x "$ADAPTER"

RESPONSE="$(printf '%s\n' '{"schema_version":2,"operation":"capabilities"}' | "$ADAPTER")"
python3 - "$RESPONSE" <<'PY'
import json
import sys

response = json.loads(sys.argv[1])
if response.get("schema_version") != 2:
    raise SystemExit(f"unexpected Apple adapter schema: {response!r}")
capabilities = response.get("capabilities")
if not isinstance(capabilities, list):
    raise SystemExit(f"Apple adapter capabilities must be a list: {response!r}")
if sorted(capabilities) != ["photographic-styles", "portrait"]:
    raise SystemExit(f"unexpected Apple adapter capabilities: {capabilities!r}")
PY

# Persistent transport must answer successive requests without waiting for stdin EOF.
# This behavioral regression caught a real deadlock caused by one-shot-style buffering.
python3 - "$ADAPTER" <<'PY'
import json
import select
import subprocess
import sys

adapter = sys.argv[1]
process = subprocess.Popen(
    [adapter, "--persistent-json-lines"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)
assert process.stdin is not None
assert process.stdout is not None
assert process.stderr is not None
request = json.dumps({"schema_version": 2, "operation": "capabilities"}).encode() + b"\n"
try:
    for index in range(2):
        process.stdin.write(request)
        process.stdin.flush()
        ready, _, _ = select.select([process.stdout], [], [], 5.0)
        if not ready:
            raise SystemExit(
                f"persistent Apple adapter did not answer request {index + 1} while stdin remained open"
            )
        response = json.loads(process.stdout.readline())
        if response.get("schema_version") != 2:
            raise SystemExit(f"unexpected persistent Apple adapter response: {response!r}")
        if sorted(response.get("capabilities", [])) != ["photographic-styles", "portrait"]:
            raise SystemExit(f"unexpected persistent Apple adapter capabilities: {response!r}")
    process.stdin.close()
    status = process.wait(timeout=5.0)
    if status != 0:
        raise SystemExit(
            f"persistent Apple adapter exited with {status}: "
            + process.stderr.read().decode(errors="replace")
        )
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
PY

TEST_INPUT="$PWD/fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"
test -f "$TEST_INPUT"

# This gate proves the committed Rust source can actually compose with the
# Swift adapter. Use a fresh target directory so a restored Cargo cache cannot
# make an uncompiled or previously different runtime source appear green.
TEST_TARGET="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-apple-adapter.XXXXXX")"
trap 'rm -rf "$TEST_TARGET"' EXIT

SCHEDULER_TEST="$TEST_TARGET/vision-request-scheduling-regression"
swiftc Sources/XDRemuxAppleAdapter/VisionRequestScheduling.swift \
  scripts/vision_request_scheduler_regression.swift \
  -o "$SCHEDULER_TEST"
"$SCHEDULER_TEST"

CARGO_TARGET_DIR="$TEST_TARGET" \
XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE="$ADAPTER" \
XDREMUX_APPLE_ADAPTER_TEST_INPUT="$TEST_INPUT" \
  cargo test --locked -p xdremux-runtime \
    --test apple_adapter \
    --test apple_portrait_rend

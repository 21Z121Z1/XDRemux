#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Rust Apple Styles consumer integration requires macOS" >&2
  exit 2
fi

swift build --product xdremux-apple-adapter
adapter="$(swift build --show-bin-path)/xdremux-apple-adapter"
test -x "$adapter"

test_root="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-rust-style-consumer.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT
oracle_output="$test_root/oracle"
mkdir -p "$oracle_output"

# The existing Swift smoke is used only as a temporary fixture producer. It
# deliberately selects the deterministic identity producer on this hosted
# machine, so this command is not solver or product-path acceptance evidence.
XDREMUX_STYLE_RUNNER_OUTPUT="$oracle_output" \
  swift test --filter PhotographicStylesRunnerSmokeTests

style_metadata="$test_root/style-metadata.bplist"
style_data="$test_root/expected-rust-identity-style-data.bin"
python3 - "$oracle_output/coloros16-live-styles.heic" "$style_metadata" "$style_data" <<'PY'
import hashlib
import importlib.util
import sys
from pathlib import Path

source_path, metadata_path, style_data_path = sys.argv[1:]
spec = importlib.util.spec_from_file_location("inspect_oppo_heif", "scripts/inspect_oppo_heif.py")
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)
source = Path(source_path).read_bytes()
meta = next(box for box in module.boxes(source, 0, len(source)) if box["type"] == "meta")
children = {box["type"]: box for box in module.boxes(source, meta["payload_start"] + 4, meta["end"])}
items = module.parse_iinf(source, children["iinf"])
locations = module.parse_iloc(source, children["iloc"])
idat = children.get("idat")
for item_id, item in items.items():
    if item.get("type") == "uri ":
        metadata = module.payload_for_item(source, locations[item_id], idat)
        Path(metadata_path).write_bytes(metadata)
        break
else:
    raise SystemExit("Styles smoke output has no uri style metadata item")

# This is the same fixed identity layout validated by xdremux-engine. The
# script uses it only as an expected byte fixture for the Rust protocol test.
block = b"".join((b"\x00\x00" if index not in {3, 7, 11} else b"\x00\x3c") for index in range(30))
style_data = block * (12 * 9 * 8)
if len(style_data) != 51840:
    raise SystemExit(f"unexpected identity style-data length: {len(style_data)}")
expected_sha = "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe"
if hashlib.sha256(style_data).hexdigest() != expected_sha:
    raise SystemExit("identity style-data fixture does not match the Rust layout digest")
Path(style_data_path).write_bytes(style_data)
print(f"rust-style-fixtures: metadata={len(metadata)} style_data={len(style_data)}")
PY

test_input="$repo_root/fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"
test -f "$test_input"
CARGO_TARGET_DIR="$test_root/cargo-target" \
XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE="$adapter" \
XDREMUX_APPLE_ADAPTER_TEST_INPUT="$test_input" \
XDREMUX_APPLE_STYLE_METADATA_INPUT="$style_metadata" \
XDREMUX_APPLE_STYLE_DATA_EXPECTED="$style_data" \
  cargo test --locked -p xdremux-runtime --test apple_adapter

echo "PASS Rust Styles metadata consumer bridge"

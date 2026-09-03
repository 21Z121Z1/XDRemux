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

cargo build --locked -p xdremux-cli
cli="$repo_root/target/debug/xdremux"
test -x "$cli"

# This is a real Rust product-path check. The fixture is a checked-in ProXDR
# source; no Swift product writer or legacy Styles runner is used to produce
# the output under test.
fixture="$repo_root/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
test -s "$fixture"

test_root="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-rust-style-consumer.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT
output="$test_root/rust-styles.heic"
inspect="$test_root/inspect.json"

XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" convert --input "$fixture" --output "$output" --apple-styles
test -s "$output"

# The canonical Rust validator checks the ISO Gain Map graph and publication
# shape. It is intentionally kept separate from the Apple consumer facts
# below.
"$cli" validate "$output" >/dev/null

python3 - "$adapter" "$output" <<'PY'
import json
import subprocess
import sys

adapter, output = sys.argv[1:]

def request(operation):
    payload = json.dumps(
        {
            "schema_version": 1,
            "operation": operation,
            "input_path": output,
        }
    ).encode() + b"\n"
    result = subprocess.run(
        [adapter], input=payload, check=True, capture_output=True
    )
    return json.loads(result.stdout)

auxiliary = request("imageio-auxiliary-facts")["auxiliary"]
required = [
    "iso_gain_map",
    "portrait_effects_matte",
    "skin_matte",
]
missing = [key for key in required if auxiliary.get(key) is not True]
if missing:
    raise SystemExit(
        f"{output}: missing ImageIO Styles consumer facts {missing}: {auxiliary!r}"
    )

gain_map = request("imageio-gain-map-facts")["gain_map"]
if gain_map["width"] <= 0 or gain_map["height"] <= 0:
    raise SystemExit(f"{output}: invalid ImageIO gain-map facts: {gain_map!r}")
print(
    "rust-styles-consumer-pass: "
    f"iso_gain_map={auxiliary['iso_gain_map']} "
    f"portrait_effects_matte={auxiliary['portrait_effects_matte']} "
    f"skin_matte={auxiliary['skin_matte']} "
    f"gain_map={gain_map['width']}x{gain_map['height']}"
)
PY

python3 scripts/inspect_oppo_heif.py "$output" --json > "$inspect"
python3 - "$inspect" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as stream:
    report = json.load(stream)
items = report["items"]
if not any(item["type"] == "uri " for item in items):
    raise SystemExit("Rust Styles output has no URI metadata item")
if not any(item["type"] == "grid" for item in items):
    raise SystemExit("Rust Styles output has no auxiliary grid item")
if not report["references"]:
    raise SystemExit("Rust Styles output has no item references")
print(
    "rust-styles-graph-pass: "
    f"items={len(items)} references={len(report['references'])}"
)
PY

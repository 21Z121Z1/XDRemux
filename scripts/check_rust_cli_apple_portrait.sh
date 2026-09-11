#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Rust Apple Portrait CLI integration requires macOS" >&2
  exit 2
fi

swift build --product xdremux-apple-adapter
adapter="$(swift build --show-bin-path)/xdremux-apple-adapter"
test -x "$adapter"

cargo build --locked -p xdremux-cli
cli="$repo_root/target/debug/xdremux"
test -x "$cli"

fixture="$repo_root/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
test -f "$fixture"

test_root="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-rust-portrait-cli.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

convert_output="$test_root/convert.heic"
XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" convert --input "$fixture" --output "$convert_output" --apple-portrait
XDREMUX_APPLE_ADAPTER="$adapter" "$cli" validate "$convert_output"

batch_output_dir="$test_root/batch"
mkdir -p "$batch_output_dir"
XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" batch --input "$fixture" --output-dir "$batch_output_dir" \
  --apple-portrait --jobs 2 --json
batch_output="$batch_output_dir/uhdr-portrait-01.xdremux.heic"
test -f "$batch_output"
XDREMUX_APPLE_ADAPTER="$adapter" "$cli" validate "$batch_output"

python3 - "$adapter" "$convert_output" "$batch_output" <<'PY'
import json
import subprocess
import sys

adapter, *outputs = sys.argv[1:]
required = [
    "iso_gain_map",
    "disparity",
    "portrait_effects_matte",
    "skin_matte",
    "hair_matte",
    "teeth_matte",
    "glasses_matte",
    "focus_metadata",
]
for output in outputs:
    request = json.dumps(
        {
            "schema_version": 2,
            "operation": "imageio-auxiliary-facts",
            "input_path": output,
        }
    ).encode() + b"\n"
    response = subprocess.run(
        [adapter], input=request, check=True, capture_output=True
    )
    facts = json.loads(response.stdout)["auxiliary"]
    missing = [key for key in required if facts.get(key) is not True]
    if missing:
        raise SystemExit(f"{output}: missing ImageIO Portrait facts {missing}: {facts!r}")
    print(f"rust-portrait-consumer-pass: {output}")
PY

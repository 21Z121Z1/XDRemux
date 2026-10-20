#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
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

# Decode the actual VideoToolbox resource from RGB UHDR and mono LHDR Styles
# conversions. This fails if the producer writes SDR codes as the reference.
# Native Main10/Log roundtrip tests run with isolated temporary output.
# Test media are not Photos acceptance files and must not enter the artifact.
XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE="$adapter" \
  cargo test --locked -p xdremux-runtime --lib \
  real_styles_thumbnail_round_trips_hdr_log_through_videotoolbox -- --nocapture

# This is a real Rust product-path check. The fixture is a checked-in ProXDR
# source; no Swift product writer or legacy Styles runner is used to produce
# the output under test.
fixture="$repo_root/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
test -s "$fixture"

test_root="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-rust-style-consumer.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

# Capture effective HEVC format-description tags from the native encoder.
# This probe intentionally does not change the existing private Styles carrier
# color interpretation; comparison to an Apple-authored thumbnail is still needed.
python3 - "$adapter" "$test_root" <<'PY'
import json
from pathlib import Path
import subprocess
import sys

adapter, root = sys.argv[1:]
root = Path(root)
raw = root / "color-probe-rgb8.bin"
raw.write_bytes(bytes([128, 128, 128]) * (64 * 64))
request = {
    "schema_version": 2,
    "operation": "videotoolbox-encode-main10",
    "input_path": str(raw),
    "output_path": str(root / "color-probe.hevc"),
    "video_toolbox_main10": {
        "raw_width": 64, "raw_height": 64, "raw_bytes_per_row": 192,
        "quality": 0.85, "hvcc_path": str(root / "color-probe.hvcc"),
    },
}
process = subprocess.run(
    [adapter], input=(json.dumps(request) + "\n").encode(),
    check=True, capture_output=True,
)
facts = json.loads(process.stdout)["video_toolbox_main10"]
for name in ("color_primaries", "transfer_function", "ycbcr_matrix"):
    print("styles-main10-observed-%s=%s" % (name, facts.get(name, "<unspecified>")))
assert (root / "color-probe.hevc").stat().st_size > 0
assert (root / "color-probe.hvcc").stat().st_size > 0
PY

output="$test_root/rust-styles.heic"
inspect="$test_root/inspect.json"
batch_output_dir="$test_root/batch"
batch_output="$batch_output_dir/uhdr-portrait-01.xdremux.heic"

XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" convert --input "$fixture" --output "$output" --apple-styles
test -s "$output"

mkdir -p "$batch_output_dir"
XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" batch --input "$fixture" --output-dir "$batch_output_dir" \
  --apple-styles --jobs 2 --json > "$test_root/batch.json"
test -s "$batch_output"

# The canonical Rust validator checks the ISO Gain Map graph and publication
# shape. It is intentionally kept separate from the Apple consumer facts
# below.
"$cli" validate "$output" >/dev/null
"$cli" validate "$batch_output" >/dev/null

python3 - "$adapter" "$output" "$batch_output" <<'PY'
import json
import subprocess
import sys

adapter, *outputs = sys.argv[1:]

def request(operation, path):
    payload = json.dumps(
        {
            "schema_version": 2,
            "operation": operation,
            "input_path": path,
        }
    ).encode() + b"\n"
    result = subprocess.run(
        [adapter], input=payload, check=True, capture_output=True
    )
    return json.loads(result.stdout)

for output in outputs:
    properties = request("imageio-image-properties", output)["image_properties"]
    smart_style = properties.get("maker_apple_integer_dictionaries", {}).get("84", {})
    expected = dict(zip(map(str, range(8)), (1, 0, 0, 1, 1, 1, 4, 0)))
    if any(smart_style.get(key) != value for key, value in expected.items()):
        raise SystemExit(f"{output}: ImageIO did not retain SmartStyle MakerApple 84: {smart_style!r}")
    auxiliary = request("imageio-auxiliary-facts", output)["auxiliary"]
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

    gain_map = request("imageio-gain-map-facts", output)["gain_map"]
    if gain_map["width"] <= 0 or gain_map["height"] <= 0:
        raise SystemExit(f"{output}: invalid ImageIO gain-map facts: {gain_map!r}")
    print(
        "rust-styles-consumer-pass: "
        f"output={output} "
        f"maker_apple_84={smart_style!r} "
        f"iso_gain_map={auxiliary['iso_gain_map']} "
        f"portrait_effects_matte={auxiliary['portrait_effects_matte']} "
        f"skin_matte={auxiliary['skin_matte']} "
        f"gain_map={gain_map['width']}x{gain_map['height']}"
    )
PY

python3 scripts/diagnostics/inspect_oppo_heif.py "$output" --json > "$inspect"
batch_inspect="$test_root/batch-inspect.json"
python3 scripts/diagnostics/inspect_oppo_heif.py "$batch_output" --json > "$batch_inspect"
python3 - "$inspect" "$batch_inspect" <<'PY'
import json
import sys

for path in sys.argv[1:]:
    with open(path, encoding="utf-8") as stream:
        report = json.load(stream)
    items = report["items"]
    if not any(item["type"] == "uri " for item in items):
        raise SystemExit(f"{path}: Rust Styles output has no URI metadata item")
    if not any(item["type"] == "grid" for item in items):
        raise SystemExit(f"{path}: Rust Styles output has no auxiliary grid item")
    if not report["references"]:
        raise SystemExit(f"{path}: Rust Styles output has no item references")
    print(
        "rust-styles-graph-pass: "
        f"inspect={path} items={len(items)} references={len(report['references'])}"
    )
PY

# The Rust product path must also carry source-derived Styles policy. A
# structurally valid graph with the fixed identity key-1 or fixed identity GTC
# would only prove container admission, not migration of the producer policy.
python3 - "$output" "$inspect" "$batch_output" "$batch_inspect" <<'PY'
import hashlib
import json
import plistlib
import struct
import sys
from scripts.diagnostics.inspect_oppo_heif import boxes, payload_for_item

output, inspect_path, batch_output, batch_inspect_path = sys.argv[1:]

def read_style_plist(path, inspect_path):
    report = json.load(open(inspect_path, encoding="utf-8"))
    metadata = next(
        item
        for item in report["items"]
        if item.get("type") == "uri " and item.get("name") == "styleMetadata"
    )
    with open(path, "rb") as stream:
        data = stream.read()
    meta = next(box for box in boxes(data, 0, len(data)) if box["type"] == "meta")
    children = boxes(data, meta["payload_start"] + 4, meta["end"])
    idat = next((child for child in children if child["type"] == "idat"), None)
    payload = payload_for_item(data, metadata["location"], idat)
    return plistlib.loads(payload)

def srgb_encode(linear):
    return linear * 12.92 if linear <= 0.0031308 else 1.055 * linear ** (1 / 2.4) - 0.055

identity_samples = [
    0 if index == 0 else 65534 if index == 255 else round(srgb_encode(index / 255) * 65534)
    for index in range(256)
] + [65534]
identity_gtc = struct.pack("<H", 257) + b"".join(
    struct.pack("<H", value) for value in identity_samples
)
identity_style_sha256 = "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe"
for path, inspect_path in ((output, inspect_path), (batch_output, batch_inspect_path)):
    plist = read_style_plist(path, inspect_path)
    style_data = plist["1"]
    if hashlib.sha256(style_data).hexdigest() == identity_style_sha256:
        raise SystemExit(f"{path}: Rust Styles output still uses the fixed identity key-1 payload")
    if plist["3"] == identity_gtc:
        raise SystemExit(f"{path}: Rust Styles output still uses the fixed identity GTC")
    if len(plist["3"]) != 516 or plist["4"] <= 0 or plist["h"] <= 0 or plist["i"]["Gain"] <= 0:
        raise SystemExit(f"{path}: Rust Styles output has invalid source-derived exposure metadata")
    print(
        "rust-styles-policy-pass: "
        f"output={path} style_sha256={hashlib.sha256(style_data).hexdigest()} "
        f"baseline_exposure={plist['4']} base_gain={plist['h']} "
        f"scene_type={plist['5']}"
    )
PY

# Trial output is the only file in the downloadable acceptance artifact.
# A v16 metadata root is a version-only hypothesis, not proven Photos support.
if [[ -n "${XDREMUX_STYLES_ACCEPTANCE_DIR:-}" ]]; then
  acceptance="$XDREMUX_STYLES_ACCEPTANCE_DIR"
  mkdir -p "$acceptance"
  master="$repo_root/fixtures/proxdr/oppo/find-x9-ultra/uhdr-master-v2-01.heic"
  target="$acceptance/uhdr-master-v2-01-apple-styles-v16.heic"
  test ! -e "$target"
  XDREMUX_APPLE_ADAPTER="$adapter" \
    "$cli" convert --input "$master" --output "$target" --apple-styles
  test -s "$target"
  "$cli" validate "$target" >/dev/null

  inspection="$test_root/master-inspect.json"
  python3 scripts/diagnostics/inspect_oppo_heif.py "$target" --json > "$inspection"
  python3 - "$target" "$inspection" <<'PY'
import hashlib
import json
import pathlib
import plistlib
import sys

heic = pathlib.Path(sys.argv[1])
inspection = json.loads(pathlib.Path(sys.argv[2]).read_text())
metadata = [item for item in inspection["items"]
            if item.get("type") == "uri " and item.get("name") == "styleMetadata"]
idats = [box for box in inspection["meta_children"] if box["type"] == "idat"]
if len(metadata) != 1 or len(idats) != 1:
    raise SystemExit("expected one Styles metadata item backed by one idat")
extents = metadata[0]["location"]["extents"]
if len(extents) != 1:
    raise SystemExit("Styles metadata must have one contiguous extent")
raw = heic.read_bytes()
extent = extents[0]
start = idats[0]["start"] + 8 + extent["offset"]
end = start + extent["length"]
if start < 0 or end > len(raw) or end <= start:
    raise SystemExit("Styles metadata extent outside HEIF")
plist = plistlib.loads(raw[start:end])
if plist.get("0") != 16:
    raise SystemExit(f"not a v16 Styles carrier: {plist.get('0')!r}")
if len(plist.get("1", b"")) != 51840 or len(plist.get("3", b"")) != 516:
    raise SystemExit("Styles payload missing expected key1 or GTC")
print(f"v16-photos-trial: file={heic.name} bytes={len(raw)} "
      f"sha256={hashlib.sha256(raw).hexdigest()} "
      "Photos Tone/Color admission remains UNVERIFIED")
PY
  files=( "$acceptance"/* )
  if [[ "${#files[@]}" -ne 1 || ! -f "${files[0]}" ]]; then
    echo "Only one final Photos trial HEIC may enter the acceptance artifact" >&2
    exit 1
  fi
fi

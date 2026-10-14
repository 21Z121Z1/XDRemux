#!/usr/bin/env bash
# Research only: version field A/B on a real Rust-generated Styles carrier.
set -euo pipefail
cd "$(dirname "$0")/../.."
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Styles native-parser experiment requires macOS" >&2
  exit 2
fi
swift build --product xdremux-apple-adapter
adapter="$(swift build --show-bin-path)/xdremux-apple-adapter"
cargo build --locked -p xdremux-cli
cli="$PWD/target/debug/xdremux"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fixture="$PWD/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
original="$work/styles-v15.heic"
changed="$work/styles-v16-version-only.heic"
XDREMUX_APPLE_ADAPTER="$adapter" "$cli" convert \
  --input "$fixture" --output "$original" --apple-styles
python3 scripts/diagnostics/inspect_oppo_heif.py "$original" --json > "$work/inspect-v15.json"
python3 research/oppo_styles/tools/probe_style_version.py \
  --input "$original" --inspect "$work/inspect-v15.json" \
  --output "$changed" --report "$work/byte-change.json"
"$cli" validate "$original" >/dev/null
"$cli" validate "$changed" >/dev/null
python3 scripts/diagnostics/inspect_oppo_heif.py "$changed" --json > "$work/inspect-v16.json"
python3 - "$adapter" "$original" "$changed" "$work/inspect-v15.json" "$work/inspect-v16.json" "$work/byte-change.json" "$work/native-result.json" <<'PY'
import json
import pathlib
import plistlib
import subprocess
import sys

adapter, original, changed, inspect_old, inspect_new, receipt_path, report_path = sys.argv[1:]
receipt = json.loads(pathlib.Path(receipt_path).read_text())
def read_plist(path, inspection_path):
    inspection = json.loads(pathlib.Path(inspection_path).read_text())
    meta = next(i for i in inspection["items"]
                if i.get("type") == "uri " and i.get("name") == "styleMetadata")
    idat = next(b for b in inspection["meta_children"] if b["type"] == "idat")
    extent = meta["location"]["extents"][0]
    data = pathlib.Path(path).read_bytes()
    start = idat["start"] + 8 + extent["offset"]
    return data[start:start + extent["length"]]

def call(operation, **kwargs):
    request = {"schema_version": 2, "operation": operation, **kwargs}
    proc = subprocess.run([adapter], input=(json.dumps(request) + "\n").encode(),
                          capture_output=True, check=False)
    if proc.returncode:
        return {"process_returncode": proc.returncode,
                "stderr": proc.stderr.decode(errors="replace")[-500:]}
    return json.loads(proc.stdout)

results = {}
for name, path, inspect in (("v15", original, inspect_old), ("v16_version_only", changed, inspect_new)):
    blob = read_plist(path, inspect)
    plist = plistlib.loads(blob)
    plist_path = pathlib.Path(report_path).with_name(name + ".bplist")
    plist_path.write_bytes(blob)
    facts = call("semantic-style-properties-facts", input_path=str(plist_path),
                 output_path=str(plist_path) + ".key1")
    auxiliary = call("imageio-auxiliary-facts", input_path=path)
    results[name] = {"styles_version": plist["0"], "native_styles_parser": facts.get("style_properties", facts),
                     "imageio_auxiliary": auxiliary.get("auxiliary", auxiliary)}
assert results["v15"]["styles_version"] == 15
assert results["v16_version_only"]["styles_version"] == 16
report = {"scope": "version-only macOS native parser, NOT a Photos Tone/Color test",
          "byte_change": receipt, "cases": results,
          "question": "Does native NeutrinoCore admit the same HEIF metadata when only the version changes?"}
pathlib.Path(report_path).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
print(json.dumps(report, sort_keys=True))
PY
if printenv XDREMUX_STYLE_VERSION_PROBE_REPORT >/dev/null; then
  cp "$work/native-result.json" "$XDREMUX_STYLE_VERSION_PROBE_REPORT"
fi

# Only the public, checked-in OPPO fixture is used for this export. The
# temporary working directory is still deleted after the validated outputs
# have been copied to the Actions artifact staging directory.
if [[ -n "${XDREMUX_STYLE_VERSION_PROBE_MEDIA_DIR:-}" ]]; then
  media_dir="$XDREMUX_STYLE_VERSION_PROBE_MEDIA_DIR"
  mkdir -p "$media_dir"
  for name in styles-v15.heic styles-v16-version-only.heic native-result.json README.txt; do
    if [[ -e "$media_dir/$name" ]]; then
      echo "Refusing to overwrite an existing Photos-import artifact: $name" >&2
      exit 1
    fi
  done
  cp "$original" "$media_dir/styles-v15.heic"
  cp "$changed" "$media_dir/styles-v16-version-only.heic"
  cp "$work/native-result.json" "$media_dir/native-result.json"
  cat > "$media_dir/README.txt" <<'README'
XDRemux / Apple Photos Photographic Styles A/B experiment
Source: checked-in public OPPO Find X9 Ultra fixture
  fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic

styles-v15.heic
  Real XDRemux --apple-styles conversion with reconstructed HDR Log thumbnail.
styles-v16-version-only.heic
  The exact same HEIC with ONLY the root Styles metadata version byte
  changed from 15 to 16. It is NOT a complete native iOS 27 Texture Styles
  asset, nor does it contain the additional v16 Texture Style resources.
native-result.json
  Hashes, byte-change offset, and the macOS ImageIO/NeutrinoCore parser
  comparison. Both files passed the Rust HEIF structural validator.

Import each HEIC separately into iPhone Photos. Compare whether the
Photographic Styles Tone/Color controls appear, whether they actually
change the picture, and whether edits persist after Save/reopen.
Compare on the same iPhone / iOS version. A parsing success or UI button
alone is NOT sufficient to prove functional Tone/Color adjustments.

These files are research-only and should not replace the product output.
README
  test -s "$media_dir/styles-v15.heic"
  test -s "$media_dir/styles-v16-version-only.heic"
  test -s "$media_dir/native-result.json"
  test -s "$media_dir/README.txt"
  echo "photos-import-artifacts-ready: $media_dir"
fi

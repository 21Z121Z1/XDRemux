#!/usr/bin/env bash
# Generate Photographic Styles HEIC samples from committed ProXDR fixtures.
# The product CLI writes Standard only. It has no Gold (or other style) selector.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cli="${XDREMUX_CLI:-$repo_root/target/debug/xdremux}"
adapter="${XDREMUX_APPLE_ADAPTER:?XDREMUX_APPLE_ADAPTER must point at xdremux-apple-adapter}"
out_root="${SAMPLE_ROOT:-$repo_root/artifacts/proxdr-styles-samples}"
product_commit="dfed3afe5e721bc8fb94faac24f5a0037559071d"
identity_key1="43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe"

test -x "$cli"
test -x "$adapter"
mkdir -p "$out_root/inspect"

{
  echo "commit $(git rev-parse HEAD)"
  echo "parent-check product $product_commit"
  git log -1 --format='subject %s'
  echo "cli $cli"
  echo "adapter $adapter"
  sw_vers
  uname -a
  rustc --version
  swift --version
  python3 --version
} >"$out_root/host.txt"

{
  echo "# Exact product commands used for each selected fixture."
  echo "# Photographic Styles generation writes Standard."
  echo "# APPLE_GENERATED_KEY1_STYLE is Standard. The CLI has no Gold selector, so no Gold file is written."
  echo
  echo "cargo build --locked -p xdremux-cli"
  echo "swift build --product xdremux-apple-adapter"
  echo "export XDREMUX_APPLE_ADAPTER=\"$adapter\""
  echo
  echo "\"$cli\" --lang en inspect --json INPUT"
  echo "XDREMUX_APPLE_ADAPTER=\"\$XDREMUX_APPLE_ADAPTER\" \"$cli\" --lang en convert --input INPUT --output OUTPUT --apple-styles"
  echo "\"$cli\" --lang en inspect --json OUTPUT"
  echo "\"$cli\" --lang en validate --json OUTPUT"
  echo "XDREMUX_APPLE_ADAPTER=\"\$XDREMUX_APPLE_ADAPTER\" \"$cli\" --lang en validate --json --apple-styles OUTPUT"
  echo "python3 scripts/diagnostics/inspect_oppo_heif.py OUTPUT --json"
} >"$out_root/commands.txt"

probe_dir="${RUNNER_TEMP:-/tmp}/proxdr-gainmap-probe"
rm -rf "$probe_dir"
mkdir -p "$probe_dir/src"
cat >"$probe_dir/Cargo.toml" <<EOF
[package]
name = "proxdr-gainmap-probe"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
xdremux-container = { path = "$repo_root/crates/xdremux-container" }
xdremux-format = { path = "$repo_root/crates/xdremux-format" }
EOF
cat >"$probe_dir/src/main.rs" <<'EOF'
use std::env;
use std::fs;
use std::process::ExitCode;

use xdremux_container::extract;
use xdremux_format::probe_jpeg_frame_profile;

fn main() -> ExitCode {
    let mut failed = false;
    for path in env::args().skip(1) {
        let data = match fs::read(&path) {
            Ok(data) => data,
            Err(error) => {
                println!("file\t{path}\terror\tread\t{error}");
                failed = true;
                continue;
            }
        };
        let extracted = match extract(&data) {
            Ok(extracted) => extracted,
            Err(error) => {
                println!("file\t{path}\terror\textract\t{error}");
                failed = true;
                continue;
            }
        };
        let frame = match probe_jpeg_frame_profile(&extracted.mask_jpeg_data) {
            Ok(frame) => frame,
            Err(error) => {
                println!(
                    "file\t{path}\tmode\t{}\tgain_map_bytes\t{}\terror\tjpeg\t{error}",
                    extracted.mode.as_str(),
                    extracted.mask_jpeg_data.len()
                );
                failed = true;
                continue;
            }
        };
        let sampling = frame
            .components
            .iter()
            .map(|component| {
                format!(
                    "{}:{}x{}",
                    component.id, component.horizontal_sampling, component.vertical_sampling
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "file\t{path}\tmode\t{}\tgain_map_bytes\t{}\tchannels\t{}\tprecision\t{}\twidth\t{}\theight\t{}\tchroma\t{:?}\tsampling\t{sampling}",
            extracted.mode.as_str(),
            extracted.mask_jpeg_data.len(),
            frame.component_count(),
            frame.precision,
            frame.width,
            frame.height,
            frame.chroma_sampling
        );
    }
    if failed { ExitCode::from(1) } else { ExitCode::SUCCESS }
}
EOF

mapfile_compat() {
  find fixtures/proxdr/oppo -name '*.heic' | sort
}

fixture_list="$out_root/fixtures.txt"
mapfile_compat >"$fixture_list"
test -s "$fixture_list"

# Same analysis PortableRuntime::analyze_proxdr uses: container extract plus JPEG SOF.
# xdremux inspect reports the ProXDR container, not the gain-map channel count.
probe_tsv="$out_root/source-channels.tsv"
# shellcheck disable=SC2046
CARGO_TARGET_DIR="${RUNNER_TEMP:-/tmp}/proxdr-gainmap-probe-target" \
  cargo run --quiet --manifest-path "$probe_dir/Cargo.toml" -- $(cat "$fixture_list") \
  >"$probe_tsv"
python3 - "$probe_tsv" "$out_root/source-channels.json" <<'PY'
import json
import sys
from pathlib import Path

rows = []
for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
    if not line.strip():
        continue
    fields = line.split("\t")
    record = {}
    if len(fields) % 2 != 0:
        record = {"raw": line, "error": "odd field count"}
    else:
        for key, value in zip(fields[0::2], fields[1::2]):
            record[key] = value
        if "channels" in record:
            record["channels"] = int(record["channels"])
        if "gain_map_bytes" in record:
            record["gain_map_bytes"] = int(record["gain_map_bytes"])
        for key in ("precision", "width", "height"):
            if key in record:
                record[key] = int(record[key])
    rows.append(record)
Path(sys.argv[2]).write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
for record in rows:
    print(
        f"{record.get('channels', '?')}\t{record.get('chroma', '')}\t{record.get('file', record.get('raw', ''))}",
        flush=True,
    )
PY

selected_list="$out_root/selected-fixtures.txt"
python3 - "$out_root/source-channels.json" "$selected_list" <<'PY'
import json
import sys
from pathlib import Path

rows = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
selected = []
for record in rows:
    path = record.get("file", "")
    lowered = path.lower()
    channels = record.get("channels")
    # 3-channel inputs are the requested set. X9 Ultra is still rendered,
    # and labelled with its real channel count, when it is not 3-channel.
    # An X8 Ultra fixture is included when one is present.
    take = channels == 3 or "/find-x9-ultra/" in lowered or "/find-x8" in lowered
    if take:
        if not isinstance(channels, int):
            raise SystemExit(f"selected fixture has no channel count: {record}")
        selected.append(f"{channels}\t{path}")
if not selected:
    raise SystemExit("no fixtures selected")
Path(sys.argv[2]).write_text("\n".join(selected) + "\n", encoding="utf-8")
print(f"selected {len(selected)} fixtures", flush=True)
PY

failures=0
note_failure() {
  failures=$((failures + 1))
  printf '%s\n' "$1" | tee -a "$out_root/failures.txt" >&2
}

run_to() {
  local slot="$1"
  shift
  set +e
  "$@" >"$slot.stdout" 2>"$slot.stderr" </dev/null
  local status=$?
  set -e
  printf '%s\n' "$status" >"$slot.exit"
  return 0
}

while IFS=$'\t' read -r channels input; do
  [[ -n "$input" ]] || continue
  relative="${input#fixtures/proxdr/oppo/}"
  device="${relative%%/*}"
  stem="$(basename "$input" .heic)"
  label="${channels}ch"
  sample_dir="$out_root/samples/$device/${stem}-gainmap-${label}"
  heic_name="${stem}-gainmap-${label}-standard-styles.heic"
  output="$sample_dir/$heic_name"
  mkdir -p "$sample_dir" "$out_root/inspect/$device"

  echo "sample $relative channels=$channels -> $heic_name" >&2
  {
    echo
    echo "# $relative ($label)"
    printf '%q ' "$cli" --lang en inspect --json "$input"
    echo
    printf '%q ' env XDREMUX_APPLE_ADAPTER="$adapter" XDREMUX_STYLES_PROFILE=1 \
      "$cli" --lang en convert --input "$input" --output "$output" --apple-styles
    echo
    printf '%q ' "$cli" --lang en inspect --json "$output"
    echo
    printf '%q ' "$cli" --lang en validate --json "$output"
    echo
    printf '%q ' env XDREMUX_APPLE_ADAPTER="$adapter" \
      "$cli" --lang en validate --json --apple-styles "$output"
    echo
    printf '%q ' python3 scripts/diagnostics/inspect_oppo_heif.py "$output" --json
    echo
  } >>"$out_root/commands.txt"

  run_to "$out_root/inspect/$device/${stem}.source-inspect" \
    "$cli" --lang en inspect --json "$input"
  cp "$out_root/inspect/$device/${stem}.source-inspect.stdout" \
    "$sample_dir/source-inspect.json" || true
  if [[ "$(cat "$out_root/inspect/$device/${stem}.source-inspect.exit")" != "0" ]]; then
    note_failure "$relative: xdremux inspect failed"
  fi

  run_to "$sample_dir/convert" \
    env XDREMUX_APPLE_ADAPTER="$adapter" XDREMUX_STYLES_PROFILE=1 \
    "$cli" --lang en convert --input "$input" --output "$output" --apple-styles
  if [[ "$(cat "$sample_dir/convert.exit")" != "0" || ! -s "$output" ]]; then
    note_failure "$relative: Photographic Styles conversion failed"
    continue
  fi

  run_to "$sample_dir/output-inspect" "$cli" --lang en inspect --json "$output"
  cp "$sample_dir/output-inspect.stdout" "$sample_dir/output-inspect.json" || true

  run_to "$sample_dir/validate" "$cli" --lang en validate --json "$output"
  cp "$sample_dir/validate.stdout" "$sample_dir/validate.json" || true
  if [[ "$(cat "$sample_dir/validate.exit")" != "0" ]]; then
    note_failure "$relative: xdremux validate failed"
  fi

  run_to "$sample_dir/validate-apple-styles" \
    env XDREMUX_APPLE_ADAPTER="$adapter" \
    "$cli" --lang en validate --json --apple-styles "$output"
  cp "$sample_dir/validate-apple-styles.stdout" "$sample_dir/validate-apple-styles.json" || true
  if [[ "$(cat "$sample_dir/validate-apple-styles.exit")" != "0" ]]; then
    note_failure "$relative: xdremux validate --apple-styles failed"
  fi

  run_to "$sample_dir/heif-inspect" \
    python3 scripts/diagnostics/inspect_oppo_heif.py "$output" --json
  cp "$sample_dir/heif-inspect.stdout" "$sample_dir/heif-inspect.json" || true
  if [[ "$(cat "$sample_dir/heif-inspect.exit")" != "0" ]]; then
    note_failure "$relative: inspect_oppo_heif failed"
  fi

  set +e
  python3 - "$adapter" "$output" "$sample_dir" "$channels" "$identity_key1" <<'PY'
import hashlib
import json
import plistlib
import subprocess
import sys
from pathlib import Path

adapter, output, sample_dir, source_channels, identity_key1 = sys.argv[1:]
sample_dir = Path(sample_dir)
output = Path(output)
source_channels = int(source_channels)
problems = []

def run_adapter(operation):
    payload = json.dumps({
        "schema_version": 2,
        "operation": operation,
        "input_path": str(output),
    }).encode() + b"\n"
    result = subprocess.run([adapter], input=payload, capture_output=True)
    slot = sample_dir / f"imageio-{operation}.json"
    slot.write_bytes(result.stdout or b"")
    (sample_dir / f"imageio-{operation}.stderr").write_bytes(result.stderr or b"")
    (sample_dir / f"imageio-{operation}.exit").write_text(str(result.returncode) + "\n", encoding="utf-8")
    if result.returncode != 0:
        problems.append(f"{operation} exit {result.returncode}: {result.stderr.decode('utf-8', 'replace')[:500]}")
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        problems.append(f"{operation} JSON: {error}")
        return None

def load_json(name):
    path = sample_dir / name
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        problems.append(f"{name}: {error}")
        return None

def validation_value(document):
    if not isinstance(document, dict):
        return None
    if document.get("valid") is not True:
        problems.append(document.get("error") or f"validation not valid: {document.get('command')}")
        return None
    details = document.get("details")
    if isinstance(details, dict) and isinstance(details.get("value"), dict):
        return details["value"]
    if isinstance(details, dict) and "channel_count" in details:
        return details
    problems.append("validation JSON has no gain-map details")
    return None

structural = validation_value(load_json("validate.json"))
apple = validation_value(load_json("validate-apple-styles.json"))
properties = run_adapter("imageio-image-properties")
auxiliary = run_adapter("imageio-auxiliary-facts")
gain_map = run_adapter("imageio-gain-map-facts")

style_facts = {
    "source_gain_map_channels": source_channels,
    "style_written": "Standard",
    "gold_output": False,
}
heif_path = sample_dir / "heif-inspect.json"
try:
    from research.oppo_styles.tools.probe_style_version import style_metadata_payload
    inspection = json.loads(heif_path.read_text(encoding="utf-8"))
    payload = style_metadata_payload(output.read_bytes(), inspection)
    plist = plistlib.loads(payload)
    key1 = plist.get("1")
    if not isinstance(key1, (bytes, bytearray)) or not key1:
        problems.append("Styles plist has no Key 1")
        key1 = b""
    digest = hashlib.sha256(key1).hexdigest()
    style_facts.update({
        "styles_version": plist.get("0"),
        "key1_bytes": len(key1),
        "key1_sha256": digest,
        "key1_is_identity": digest == identity_key1,
        "key3_bytes": len(plist.get("3", b"")) if isinstance(plist.get("3"), (bytes, bytearray)) else None,
        "baseline_exposure": plist.get("4"),
        "scene_type": plist.get("5"),
        "base_gain": plist.get("h"),
    })
    if digest == identity_key1:
        problems.append("Key 1 is the fixed identity payload")
    if len(key1) != 51840:
        problems.append(f"Key 1 length is {len(key1)}, expected 51840")
    if plist.get("0") != 15:
        problems.append(f"Styles version is {plist.get('0')!r}, expected 15")
except Exception as error:
    problems.append(f"Styles metadata: {error}")

if isinstance(structural, dict):
    style_facts["validate_channel_count"] = structural.get("channel_count")
    style_facts["validate_chroma"] = structural.get("chroma_sampling")
    style_facts["validate_gain_map_size"] = [structural.get("width"), structural.get("height")]
    style_facts["gain_map_matches_source_channels"] = structural.get("channel_count") == source_channels
if isinstance(apple, dict):
    style_facts["apple_styles_validate_channel_count"] = apple.get("channel_count")

image_properties = (properties or {}).get("image_properties") or {}
maker = (image_properties.get("maker_apple_integer_dictionaries") or {}).get("84") or {}
style_facts["maker_apple_84"] = maker
expected_prefix = {"0": 1, "1": 0, "2": 0, "3": 1, "4": 1, "5": 1, "6": 4, "7": 0}
if any(maker.get(key) != value for key, value in expected_prefix.items()):
    problems.append(f"MakerApple 84 does not match the Standard SmartStyle prefix: {maker}")

auxiliary_facts = (auxiliary or {}).get("auxiliary") or {}
style_facts["auxiliary"] = auxiliary_facts
for key in ("iso_gain_map", "portrait_effects_matte", "skin_matte"):
    if auxiliary_facts.get(key) is not True:
        problems.append(f"ImageIO auxiliary {key} is not true")

gain = (gain_map or {}).get("gain_map") or {}
pixel_format = gain.get("pixel_format")
if isinstance(pixel_format, int):
    style_facts["imageio_gain_map_pixel_format"] = pixel_format.to_bytes(4, "big").decode("ascii", "replace")
style_facts["imageio_gain_map"] = {
    "pixel_format": style_facts.get("imageio_gain_map_pixel_format"),
    "width": gain.get("width"),
    "height": gain.get("height"),
}
data = output.read_bytes()
style_facts["output_bytes"] = len(data)
style_facts["output_sha256"] = hashlib.sha256(data).hexdigest()
style_facts["problems"] = problems
(sample_dir / "styles-facts.json").write_text(json.dumps(style_facts, indent=2) + "\n", encoding="utf-8")
if problems:
    print("\n".join(problems), file=sys.stderr)
    raise SystemExit(1)
print(json.dumps({k: style_facts[k] for k in (
    "styles_version", "key1_bytes", "key1_sha256", "key1_is_identity",
    "validate_channel_count", "validate_chroma", "gain_map_matches_source_channels",
    "output_bytes",
)}, sort_keys=True))
PY
  facts_status=$?
  set -e
  if [[ "$facts_status" != "0" ]]; then
    note_failure "$relative: Styles metadata verification failed"
  fi
done <"$selected_list"

python3 - "$out_root" <<'PY'
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
samples = []
for facts_path in sorted(root.glob("samples/*/*/styles-facts.json")):
    facts = json.loads(facts_path.read_text(encoding="utf-8"))
    heics = list(facts_path.parent.glob("*.heic"))
    samples.append({
        "directory": str(facts_path.parent.relative_to(root)),
        "heic": heics[0].name if heics else None,
        "bytes": facts.get("output_bytes"),
        "sha256": facts.get("output_sha256"),
        "source_gain_map_channels": facts.get("source_gain_map_channels"),
        "validate_channel_count": facts.get("validate_channel_count"),
        "validate_chroma": facts.get("validate_chroma"),
        "gain_map_matches_source_channels": facts.get("gain_map_matches_source_channels"),
        "styles_version": facts.get("styles_version"),
        "key1_bytes": facts.get("key1_bytes"),
        "key1_sha256": facts.get("key1_sha256"),
        "key1_is_identity": facts.get("key1_is_identity"),
        "style_written": facts.get("style_written"),
        "problems": facts.get("problems") or [],
    })
channels = json.loads((root / "source-channels.json").read_text(encoding="utf-8"))
manifest = {"samples": samples, "source_channels": channels}
(root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
lines = [
    "XDRemux Photographic Styles samples",
    "Style written: Standard. The CLI cannot select Gold.",
    "",
    "Source gain-map channels",
]
for record in channels:
    lines.append(
        f"  {record.get('channels')}  {record.get('chroma')}  {record.get('mode')}  {record.get('file')}"
    )
lines.append("")
lines.append("Outputs")
for sample in samples:
    lines.append(
        f"  {sample['bytes']} bytes  channels {sample['source_gain_map_channels']}->{sample['validate_channel_count']} "
        f"{sample['validate_chroma']}  key1 {sample['key1_bytes']}  {sample['directory']}/{sample['heic']}"
    )
(root / "README.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
print("\n".join(lines))
PY

if [[ "$failures" -ne 0 ]]; then
  echo "$failures sample checks failed" >&2
  exit 1
fi
echo "all selected Styles samples passed" >&2

#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Apple device-validation bundle requires macOS" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

output_root="${1:-artifacts/apple-device-validation}"
cli="${XDREMUX_CLI:-$repo_root/target/debug/xdremux}"
adapter="${XDREMUX_APPLE_ADAPTER:-}"

if [[ -e "$output_root" ]] && [[ -n "$(find "$output_root" -mindepth 1 -print -quit 2>/dev/null)" ]]; then
  echo "refusing to overwrite a non-empty device-validation directory: $output_root" >&2
  exit 1
fi
mkdir -p "$output_root"

if [[ ! -x "$cli" ]]; then
  cargo build --locked -q -p xdremux-cli
fi
if [[ ! -x "$cli" ]]; then
  echo "Rust CLI was not built at $cli" >&2
  exit 1
fi

if [[ -z "$adapter" ]]; then
  swift build --product xdremux-apple-adapter >/dev/null
  adapter="$(swift build --show-bin-path)/xdremux-apple-adapter"
fi
if [[ ! -x "$adapter" ]]; then
  echo "Apple adapter was not built at $adapter" >&2
  exit 1
fi

source="$repo_root/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
if [[ ! -s "$source" ]]; then
  echo "missing Apple device-validation source fixture: $source" >&2
  exit 1
fi

source_sha_before="$(shasum -a 256 "$source" | awk '{print $1}')"
portrait="$output_root/portrait.heic"
styles="$output_root/photographic-styles.heic"

XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" convert --input "$source" --output "$portrait" --apple-portrait \
  >"$output_root/portrait-conversion.log" 2>&1
XDREMUX_APPLE_ADAPTER="$adapter" \
  "$cli" convert --input "$source" --output "$styles" --apple-styles \
  >"$output_root/styles-conversion.log" 2>&1

for output in "$portrait" "$styles"; do
  test -s "$output"
  "$cli" validate "$output" >"${output%.heic}-validation.txt"
done

source_sha_after="$(shasum -a 256 "$source" | awk '{print $1}')"
if [[ "$source_sha_before" != "$source_sha_after" ]]; then
  echo "Rust Apple conversion modified its source fixture: $source" >&2
  exit 1
fi

python3 - "$adapter" "$source" "$portrait" "$styles" "$output_root" <<'PY'
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys

adapter = sys.argv[1]
source = Path(sys.argv[2])
portrait = Path(sys.argv[3])
styles = Path(sys.argv[4])
output_root = Path(sys.argv[5])


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def auxiliary_facts(path: Path) -> dict[str, object]:
    request = json.dumps(
        {
            "schema_version": 2,
            "operation": "imageio-auxiliary-facts",
            "input_path": str(path),
        }
    ).encode() + b"\n"
    completed = subprocess.run(
        [adapter],
        input=request,
        check=True,
        capture_output=True,
    )
    response = json.loads(completed.stdout)
    if response.get("schema_version") != 2:
        raise SystemExit(f"unexpected Apple adapter response schema: {response!r}")
    facts = response.get("auxiliary")
    if not isinstance(facts, dict):
        raise SystemExit(f"missing ImageIO auxiliary facts: {response!r}")
    return facts


portrait_facts = auxiliary_facts(portrait)
styles_facts = auxiliary_facts(styles)
requirements = {
    "portrait": (
        portrait_facts,
        [
            "iso_gain_map",
            "disparity",
            "portrait_effects_matte",
            "skin_matte",
            "hair_matte",
            "teeth_matte",
            "glasses_matte",
            "focus_metadata",
        ],
    ),
    "photographic-styles": (
        styles_facts,
        ["iso_gain_map", "portrait_effects_matte", "skin_matte"],
    ),
}
for name, (facts, required) in requirements.items():
    missing = [key for key in required if facts.get(key) is not True]
    if missing:
        raise SystemExit(
            f"{name}: offline Apple consumer facts are missing {missing}: {facts!r}"
        )


def output_record(path: Path, facts: dict[str, object]) -> dict[str, object]:
    return {
        "path": path.name,
        "size_bytes": path.stat().st_size,
        "sha256": sha256(path),
        "imageio_auxiliary_facts": facts,
        "device_acceptance": {
            "tested": False,
            "device": None,
            "os_version": None,
            "photos_imported": None,
            "feature_recognized": None,
            "editing_ui_available": None,
            "edited_render_matches_expected_behavior": None,
            "revert_or_round_trip_succeeds": None,
            "notes": None,
        },
    }


manifest = {
    "manifest_schema_version": 1,
    "purpose": "Rust-produced Apple feature outputs for manual Photos/device acceptance",
    "source": {
        "path": str(source.relative_to(Path.cwd())),
        "size_bytes": source.stat().st_size,
        "sha256": sha256(source),
    },
    "outputs": {
        "portrait": output_record(portrait, portrait_facts),
        "photographic_styles": output_record(styles, styles_facts),
    },
    "claim_boundary": (
        "Offline Rust validation and ImageIO consumer facts passed when this bundle was built. "
        "All device_acceptance fields remain false/null until a physical-device Apple Photos "
        "test is performed and recorded."
    ),
}
(output_root / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

readme = """# Apple Photos device-validation bundle

This directory contains two HEIC files produced from the same checked-in OPPO
fixture by the canonical Rust `xdremux` CLI: one with `--apple-portrait` and one
with `--apple-styles`. Swift is used only through `xdremux-apple-adapter` for
Apple framework primitives.

The accompanying validation files and `manifest.json` record only offline
structure and ImageIO consumer facts. They do **not** prove Apple Photos device
acceptance, editability, visual equivalence, or round-trip behavior.

For each output on a representative physical Apple device, record in
`manifest.json` (or an external result linked by its SHA-256): device and OS,
Photos import success, feature recognition, editing UI availability, an actual
edit/render observation, and revert/round-trip behavior. Preserve the output
SHA-256 so the observation remains bound to the exact tested asset.
"""
(output_root / "README.md").write_text(readme, encoding="utf-8")
PY

#!/usr/bin/env bash
set -euo pipefail

fixture_root="${1:-fixtures}"
output_root="${2:-artifacts/live-photo-device-validation}"
xdremux_bin="${XDREMUX_BIN:-.build/release/xdremux}"

if [[ ! -x "$xdremux_bin" ]]; then
  swift build -c release --product xdremux
fi

rm -rf "$output_root"
mkdir -p "$output_root"

# Keep this list deliberately narrow. The device-validation bundle is for the vendor paths whose
# Live Photo geometry is under active validation, not a mirror of every generic Motion Photo
# fixture. R002/R003 are byte-identical duplicates of the two Samsung HEIF inputs and therefore do
# not add device-level coverage.
cases=(
  "ColorOS16|IMG20260710191114_ColorOS_16.jpg"
  "ColorOS16|IMG20260801190843_ColorOS_16.jpg"
  "Samsung-JPEG|20260312_135625..jpg"
  "Samsung-JPEG|20260312_135627..jpg"
  "Samsung-HEIF|20260312_135609..heic"
  "Samsung-HEIF|20260312_135610..heic"
)

sources_tsv="$output_root/sources.tsv"
printf 'vendor\tsource\toutput_stem\n' > "$sources_tsv"

for entry in "${cases[@]}"; do
  vendor="${entry%%|*}"
  filename="${entry#*|}"
  source="$fixture_root/$filename"
  if [[ ! -f "$source" ]]; then
    echo "missing required device-validation fixture: $source" >&2
    exit 1
  fi

  stem="${filename%.*}"
  # The supplied Samsung fixture names contain a deliberate extra dot before the extension. Keep
  # the source filename untouched while using a filesystem-friendly output stem.
  output_stem="${stem%.}"
  case_dir="$output_root/$vendor/$output_stem"
  mkdir -p "$case_dir"

  output_heic="$case_dir/$output_stem.heic"
  output_mov="$case_dir/$output_stem.mov"
  "$xdremux_bin" convert \
    --input "$source" \
    --output "$output_heic" \
    2>&1 | tee "$case_dir/conversion.log"

  if [[ ! -s "$output_heic" || ! -s "$output_mov" ]]; then
    echo "converter did not produce a complete Live Photo pair for $filename" >&2
    exit 1
  fi

  printf '%s\t%s\t%s\n' "$vendor" "$filename" "$output_stem" >> "$sources_tsv"
done

python3 - "$fixture_root" "$output_root" <<'PY'
from __future__ import annotations

import hashlib
import json
import pathlib
import sys

fixture_root = pathlib.Path(sys.argv[1])
output_root = pathlib.Path(sys.argv[2])


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

rows = []
with (output_root / "sources.tsv").open("r", encoding="utf-8") as stream:
    next(stream)
    for line in stream:
        vendor, source_name, output_stem = line.rstrip("\n").split("\t")
        case_dir = output_root / vendor / output_stem
        heic = case_dir / f"{output_stem}.heic"
        mov = case_dir / f"{output_stem}.mov"
        source = fixture_root / source_name
        rows.append(
            {
                "vendor": vendor,
                "source": source_name,
                "source_sha256": sha256(source),
                "pair": {
                    "still": str(heic.relative_to(output_root)),
                    "still_size": heic.stat().st_size,
                    "still_sha256": sha256(heic),
                    "motion": str(mov.relative_to(output_root)),
                    "motion_size": mov.stat().st_size,
                    "motion_sha256": sha256(mov),
                },
            }
        )

manifest = {
    "schema_version": 1,
    "purpose": "Real-device Apple Photos validation for XDRemux Motion Photo conversion",
    "pair_count": len(rows),
    "pairs": rows,
}
(output_root / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

readme = """# Live Photo device-validation bundle

Each leaf directory contains one same-basename `.heic` + `.mov` pair produced by XDRemux from a
real repository fixture. The converter writes the shared Live Photo asset identifier and
still-image-time metadata; the MOV path uses compressed-sample passthrough rather than video
re-encoding.

This archive is intentionally limited to the active ColorOS 16 and Samsung validation corpus.
`R002_...` and `R003_...` are omitted because they are byte-identical duplicates of the included
Samsung HEIF fixtures.

## Import for device testing

A Live Photo is a paired resource, not a single standalone file format. Keep each `.heic` and
`.mov` together and with the same basename. The most reliable manual route is to select both files
for one pair together in macOS Photos, confirm that Photos recognizes one Live Photo asset, and
then sync it to the iPhone through iCloud Photos or AirDrop from Photos. If a PhotoKit-based XDRemux
importer is available, it can instead add the two paired resources directly on the device.

`manifest.json` records the source identity and SHA-256 of every generated resource so a device
observation can be tied back to the exact CI output.
"""
(output_root / "README.md").write_text(readme, encoding="utf-8")
PY

rm -f "$sources_tsv"

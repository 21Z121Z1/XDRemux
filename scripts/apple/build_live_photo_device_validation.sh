#!/usr/bin/env bash
set -euo pipefail

fixture_root="${1:-fixtures}"
output_root="${2:-artifacts/live-photo-device-validation}"
cli="${XDREMUX_CLI:-target/debug/xdremux}"

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

# Keep this list deliberately narrow. The device-validation bundle is for the
# vendor paths whose Live Photo geometry is under active validation, not a
# mirror of every generic Motion Photo fixture.
cases=(
  "ColorOS16|motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg"
  "ColorOS16|motion-photo/oppo/coloros16-dualstream-ultrahdr-02.jpg"
  "Samsung-JPEG|motion-photo/samsung/jpeg-ultrahdr-01.jpg"
  "Samsung-JPEG|motion-photo/samsung/jpeg-ultrahdr-02.jpg"
  "Samsung-HEIF|motion-photo/samsung/heif-ultrahdr-01.heic"
  "Samsung-HEIF|motion-photo/samsung/heif-ultrahdr-02.heic"
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

  basename="${filename##*/}"
  stem="${basename%.*}"
  case_dir="$output_root/$vendor/$stem"
  mkdir -p "$case_dir"

  output_heic="$case_dir/$stem.heic"
  output_mov="$case_dir/$stem.mov"
  before_sha="$(shasum -a 256 "$source" | awk '{print $1}')"

  "$cli" convert --input "$source" --output "$output_heic" >"$case_dir/conversion.log" 2>&1
  "$cli" validate "$output_heic" >"$case_dir/still-validation.txt"
  "$cli" validate "$output_mov" >"$case_dir/movie-validation.txt"
  after_sha="$(shasum -a 256 "$source" | awk '{print $1}')"
  if [[ "$before_sha" != "$after_sha" ]]; then
    echo "Rust conversion modified its source fixture: $source" >&2
    exit 1
  fi

  if [[ ! -s "$output_heic" || ! -s "$output_mov" ]]; then
    echo "converter did not produce a complete Live Photo pair for $filename" >&2
    exit 1
  fi
  grep -q '^valid: live-photo$' "$case_dir/still-validation.txt"
  grep -q '^valid: live-photo$' "$case_dir/movie-validation.txt"
  printf '%s\t%s\t%s\n' "$vendor" "$filename" "$stem" >> "$sources_tsv"
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
    "purpose": "Rust-produced Live Photo pairs for manual Apple Photos/device validation",
    "pair_count": len(rows),
    "pairs": rows,
}
(output_root / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

readme = """# Live Photo device-validation bundle

Each leaf directory contains one same-basename .heic + .mov pair produced by the Rust
xdremux CLI from a real repository fixture. The converter writes the shared Live Photo asset
identifier and still-image-time metadata; the MOV path uses compressed-sample passthrough rather
than video re-encoding.

This archive is an offline/structural preparation step. It does not claim that Photos, PhotoKit,
or a physical device accepted the pair. Keep the two resources together when importing them into
Photos or a device, and record any consumer observation against manifest.json.
"""
(output_root / "README.md").write_text(readme, encoding="utf-8")
PY

rm -f "$sources_tsv"

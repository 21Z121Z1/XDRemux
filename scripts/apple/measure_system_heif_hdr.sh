#!/usr/bin/env bash
# Opt-in system gain-map measurement. Run on the xcode-27 host after the
# adapter and xdremux CLI are built. RESEARCH_ROOT is Photographic_Style_Research
# at 37dd551. The default product encoder is not used: XDREMUX_SYSTEM_GAIN_MAP
# asks Core Image to calculate the gain map.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
research_root="${RESEARCH_ROOT:?RESEARCH_ROOT is required}"
out_dir="${SYSTEM_HDR_OUT:?SYSTEM_HDR_OUT is required}"
cli="${XDREMUX_CLI:?XDREMUX_CLI is required}"
adapter="${XDREMUX_APPLE_ADAPTER:?XDREMUX_APPLE_ADAPTER is required}"
source="${repo_root}/fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic"
heic="${out_dir}/lhdr-v1-01.system.heic"
report="${out_dir}/lhdr-v1-01.system-hdr.json"

mkdir -p "$out_dir"
{
  sw_vers
  uname -a
  xcodebuild -version
  printf 'research=%s\n' "$(git -C "$research_root" rev-parse HEAD)"
  printf 'xdremux=%s\n' "$(git -C "$repo_root" rev-parse HEAD)"
} > "$out_dir/host.txt"

XDREMUX_APPLE_ADAPTER="$adapter" \
XDREMUX_SYSTEM_GAIN_MAP=1 \
XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE=256 \
XDREMUX_SYSTEM_GAIN_MAP_REPORT="$report" \
  "$cli" convert --input "$source" --output "$heic" --apple-styles
test -s "$heic"
test -s "$report"

(
  cd "$research_root"
  python3 consumer-model/harness/run_tone_sweep.py \
    "$out_dir/tone" \
    "$heic" \
    --tones -1 0 1
)

python3 "$repo_root/scripts/apple/score_system_hdr_tone.py" \
  "$out_dir/tone" \
  "$out_dir/tone-shape.json"

find "$out_dir/tone" -type f \( -name '*.f32' -o -name '*.bin' -o -name '*.f16' \) -delete

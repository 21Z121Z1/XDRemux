#!/usr/bin/env bash
# Render Apple and XDRemux photos through PISemanticStyleFilter.
# Run on the xcode-27 / 26A434 host. RESEARCH_ROOT is a checkout of
# 21Z121Z1/Photographic_Style_Research at 37dd551.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
research_root="${RESEARCH_ROOT:?RESEARCH_ROOT is required}"
out_dir="${TONE_OUT:?TONE_OUT is required}"
cli="${XDREMUX_CLI:?XDREMUX_CLI is required}"
adapter="${XDREMUX_APPLE_ADAPTER:?XDREMUX_APPLE_ADAPTER is required}"
resources="${RUNNER_TEMP:-/tmp}/photo-resources"

tones=(-1 -0.75 -0.5 -0.35 -0.15 0 0.15 0.35 0.5 0.75 1)

python3 "$repo_root/scripts/apple/check_style_tone_response.py" --self-test

mkdir -p "$out_dir"
{
  sw_vers
  uname -a
  xcodebuild -version
  printf 'research=%s\n' "$(git -C "$research_root" rev-parse HEAD)"
  printf 'xdremux=%s\n' "$(git -C "$repo_root" rev-parse HEAD)"
} > "$out_dir/host.txt"

prune_renders() {
  local root="$1"
  find "$root" -type f \( \
    -name '*.f32' -o -name 'msl-output.f16' -o -name 'msl-checkpoints.f32' \
    -o -name 'sentinel-checkpoints.f32' \
  \) -delete
  find "$root" -type f -name '*.bin' ! -name '_outputImageTexture.bin' -delete
}

(
  cd "$research_root"
  python3 consumer-model/harness/run_tone_sweep.py \
    "$out_dir/apple" \
    Apple_Native_Samples/IMG_0032.HEIC \
    Apple_Native_Samples/IMG_0033.HEIC \
    research/vm-native/samples/IMG_2600.HEIC \
    research/vm-native/samples/IMG_2604.HEIC \
    research/vm-native/samples/IMG_2608.HEIC \
    --tones "${tones[@]}"
)
prune_renders "$out_dir/apple"

python3 "$repo_root/scripts/apple/check_style_tone_response.py" \
  --sanity "$out_dir/apple/tone-sweep.json" \
  --renders "$out_dir/apple" \
  --out "$out_dir/apple-report"
trusted="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["trusted"])' "$out_dir/apple-report/tone-response.json")"
if [[ "$trusted" != "True" ]]; then
  echo "tone 0 on IMG_0032 did not match the native renderer; XDRemux numbers will not be treated as 26A434 evidence" >&2
  exit 3
fi

cd "$repo_root"
generated="$out_dir/generated"
mkdir -p "$generated"
photos=()
while IFS= read -r fixture; do
  source="$repo_root/$fixture"
  name="$(basename "${fixture%.heic}").xdremux.heic"
  output="$generated/$name"
  XDREMUX_APPLE_ADAPTER="$adapter" \
    "$cli" convert --input "$source" --output "$output" --apple-styles
  test -s "$output"
  photos+=("$output")
done <<'EOF'
fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic
fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic
fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic
EOF

(
  cd "$research_root"
  python3 consumer-model/harness/run_tone_sweep.py \
    "$out_dir/xdremux" \
    "${photos[@]}" \
    --tones "${tones[@]}"
)
prune_renders "$out_dir/xdremux"

python3 "$repo_root/scripts/apple/check_style_tone_response.py" \
  --sanity "$out_dir/apple/tone-sweep.json" \
  --renders "$out_dir/xdremux" \
  --out "$out_dir/report"

python3 "$repo_root/scripts/apple/check_style_tone_response.py" \
  --compare "$out_dir/apple-report/tone-response.json" "$out_dir/report/tone-response.json" \
  --metadata "$resources" \
  --out "$out_dir/asymmetry"

# The scored report keeps the native half textures. Drop them after the
# comparison images exist so the one-day artifact stays readable.
find "$out_dir/apple" "$out_dir/xdremux" -type f -name '_outputImageTexture.bin' -delete

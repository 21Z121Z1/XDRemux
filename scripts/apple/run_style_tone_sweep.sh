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

python3 "$repo_root/scripts/apple/check_style_tone_response.py" --self-test

mkdir -p "$out_dir"
{
  sw_vers
  uname -a
  xcodebuild -version
  printf 'research=%s\n' "$(git -C "$research_root" rev-parse HEAD)"
  printf 'xdremux=%s\n' "$(git -C "$repo_root" rev-parse HEAD)"
} > "$out_dir/host.txt"

(
  cd "$research_root"
  python3 consumer-model/harness/run_tone_sweep.py \
    "$out_dir/sanity" \
    Apple_Native_Samples/IMG_0032.HEIC \
    --tones -1 -0.35 0 0.35 1
)

python3 "$repo_root/scripts/apple/check_style_tone_response.py" \
  --sanity "$out_dir/sanity/tone-sweep.json" \
  --renders "$out_dir/sanity" \
  --out "$out_dir/sanity-report"
trusted="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["trusted"])' "$out_dir/sanity-report/tone-response.json")"
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
    --tones -1 -0.35 0 0.35 1
)

python3 "$repo_root/scripts/apple/check_style_tone_response.py" \
  --sanity "$out_dir/sanity/tone-sweep.json" \
  --renders "$out_dir/xdremux" \
  --out "$out_dir/report"

# Keep the native oracle, the model replay, and the logs. The other bound
# textures are large and are not needed to read the tone response.
find "$out_dir/sanity" "$out_dir/xdremux" -type f -name '*.bin' \
  ! -name '_outputImageTexture.bin' -delete

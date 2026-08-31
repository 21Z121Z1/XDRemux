#!/usr/bin/env bash
set -euo pipefail

output="$(cargo run --quiet --locked -p xdremux-engine --bin xdremux-engine-vectors)"
printf '%s\n' "$output"

grep -Fx 'preserve-420|family=X7|requested=Hybrid|effective=Hybrid|chroma=Yuv420|depth=8' <<<"$output" >/dev/null
grep -Fx 'promote-422|chroma=Yuv444|depth=8' <<<"$output" >/dev/null
grep -Fx 'strict-tmap|requested=Passthrough|effective=Hybrid' <<<"$output" >/dev/null
grep -F 'reject-444-to-420|no encoder capability preserves Gain Map layout GainMapCodecLayout { chroma: Yuv444' <<<"$output" >/dev/null
grep -F 'reject-10-to-8|no encoder capability preserves Gain Map layout GainMapCodecLayout { chroma: Yuv444, luma_bit_depth: 10' <<<"$output" >/dev/null
grep -Fx 'missing-decoder|missing required operation capabilities: RasterDecoder(Jpeg)' <<<"$output" >/dev/null
grep -Fx 'missing-styles|missing required operation capabilities: PhotographicStylesAdapter' <<<"$output" >/dev/null

echo 'PASS Rust conversion planner integration vectors'

# OPPO Portrait Depth Header and Disparity Scale

Date: 2026-07-13

## Corpus

`scripts/analyze_oppo_portrait_depth_corpus.py` inspected 80 unique-name
camera-original rear-portrait HEIC files under the local OPPO import folder.
Every included row has `rear.depth`, `rear.depth.config`, and `src.image`.

The corpus covers:

| Equivalent focal length | Sample count |
|---:|---:|
| 23 | 9 |
| 41 | 1 |
| 47 | 12 |
| 59 | 2 |
| 70 | 23 |
| 85 | 4 |
| 90/98/125 | 1 each |
| 139 | 5 |
| 164/206 | 1 each |
| 230 | 19 |

The derived CSV is intentionally local because it contains filenames and
hashes for private source photos. Reproduce it with:

```bash
python3 scripts/analyze_oppo_portrait_depth_corpus.py \
  "/path/to/OPPO originals" \
  /private/tmp/oppo_portrait_depth_metrics.csv
```

## Header fields used

After zstd decompression, `rear.depth` begins with a 768-byte header followed
by the uint8 rank plane. Across all 80 samples:

- offsets `0x00/0x04`: rank-plane width and height;
- offset `0x18`: per-rank disparity scale;
- offset `0x1c`: effective focal length in rank-plane pixels;
- offset `0x20`: lens/depth-pair baseline profile.

The focal field changes continuously rather than only at optical switch
points. Median observations include:

| Equivalent mm | Header fx at depth resolution | fx / equivalent mm | Baseline |
|---:|---:|---:|---:|
| 23 | 725.10 | 31.53 | 27.64 |
| 41 | 1286.26 | 31.37 | 27.64 |
| 47 | 1476.25 | 31.41 | 27.64 |
| 59 | 1794.90 | 30.42 | 27.64 |
| 70 | 2041.01 | 29.16 | 38.84 |
| 85 | 2490.37 | 29.30 | 38.84 |
| 139 | 4098.02 | 29.48 | 38.84 |
| 206 | 6111.09 | 29.67 | 38.84 |
| 230 | 6892.71 | 29.97 | 30.35 |

Therefore a non-optical digital-zoom focal length does not require
interpolation between a hand-maintained lens table. The header fx records the
real source-depth geometry continuously. It is retained for diagnostics but is
not injected into a mismatched private Apple `REND` renderer.

This statement is limited to source geometry. X9 Ultra firmware reverse
engineering shows that OPPO's later bokeh renderer separately uses a monotonic
`Phone_Len` / `Hasselblad_Len_Table_phone` render table and different nonlinear
1-2x, 2-3x, 3-6x, and 6-10x CoC branches. The table does not replace the
per-capture header scale; it shapes the renderer after focus depth is selected.
See `oppo-x9-ultra-portrait-depth-consumption-20260713.md`.

## Rank-to-disparity conversion

The previous converter used a fixed interval `1.4...4.3`, or a fixed step of
`2.9/255 = 0.01137` per uint8 rank. The real header scale ranges approximately
from `0.0011` to `0.0051`, so the old conversion enlarged depth differences by
roughly 2.2x to 10x.

The corrected conversion is:

```text
deltaDisparity = deltaRank * headerScale[0x18]
disparity(rank) = 1.4 + (255 - rank) * headerScale[0x18]
```

The constant 1.4 is only an offset; relative/high disparity rendering is
driven by differences. The per-capture header controls the span.

## Distance behavior and 230mm evidence

Raw rank percentiles are not monotonic with `rear.depth.config.distance`
across unrelated scenes. Rank values are scene-relative, and scene composition
changes the observed percentile span. Distance alone must not be used to invent
a disparity interval.

The 19 real 230mm portraits provide a stronger controlled result:

- all use header fx `6892.71` at `1024x768` and baseline `30.35`;
- six captures with config distance `259...605` use header scale
  `0.00416...0.00512` (median `0.00498`);
- thirteen captures with distance `795...29766` use header scale
  `0.00113...0.00122` (median `0.00117`);
- their converted p99-p01 disparity spans have medians about `1.13` and
  `0.26`, respectively.

After transforming config focus coordinates from the portrait `900x1200`
space into the Orientation-6 landscape rank plane, the local 21x21 focus-rank
median provides an additional check. For the 19 samples,
`(255 - focusRank) * headerScale` has correlation approximately `0.795` with
`1 / configDistance`. This is the expected direction for disparity and is much
stronger than the relationship obtained from raw rank alone.

Thus disparity differences do change with subject distance/depth mode, but the
change is encoded per capture in the header scale. The correct calculation is
not a direct distance formula and is not restricted to optical focal points.

## Continuous Apple render-domain gain

Device testing established two renderer endpoints after source disparity was
corrected: canonical Apple calibration at 1x was too weak, while physical
OPPO-derived auxiliary calibration was too strong. The next probe keeps the
canonical calibration/REND pair and applies the geometric midpoint of their
effective-focal-length ratio to disparity amplitude:

```text
renderGain = sqrt(sourceEffectiveDepthFx / canonicalEffectiveRenderFx)
renderedDelta = deltaRank * headerScale * renderGain
```

This is not RGB-guided depth super-resolution and does not change depth edges.
It is a scalar renderer-domain mapping applied to the original `1024x768`
rank-derived disparity. RGB guidance is independently limited to PEM/hair
boundary refinement.

Across the 80 real portraits, the gain varies continuously from about `1.00x`
at 23mm to `3.08x` at 230mm. The source p99-p01 span and rendered span have
correlations of approximately `-0.54` and `0.24` with equivalent focal length,
respectively. This directly contradicts a model where long focal length alone
must make disparity or blur grow linearly.

The 230mm subset is especially important. Its gain is the same `3.0803x`
because header fx is stable, but the per-capture header scale separates the
near group from the longer-distance group:

- distance `259...605`: source span about `0.99...1.27`, rendered span about
  `3.04...3.92`;
- distance `795...29766`: source span about `0.16...0.28`, rendered span about
  `0.48...0.86`.

Thus the previous excessive long-focal blur came from feeding physical
70-230mm auxiliary calibration into a fixed private Apple render graph, not
from the OPPO depth plane intrinsically growing with focal length. The new
gain is sublinear in focal geometry and leaves real distance-mode variation in
the source header scale.

## Remaining boundary

Apple `REND` rendering parameters come from the device-consumable donor
profile and therefore remain paired with that donor's auxiliary camera
calibration. Device testing proved that mixing this graph with physical OPPO
70-230mm calibration still over-amplifies f/16 blur after rank-scale
correction. The current test also synchronizes the independently identified
`REND` aperture record `0x012f`; all other private fields remain unchanged.

The corresponding canonical-calibration device pass then produced too little
visible blur even at f/1.4. Pass D now probes the continuous geometric midpoint
between those endpoints: header scale remains the source-disparity conversion,
while a separate Apple-domain gain controls final Photos blur amplitude. Its
offline structure and payload-preservation gates pass; device acceptance is
pending. See
`oppo-apple-portrait-blur-strength-bracketing-20260713.md`.

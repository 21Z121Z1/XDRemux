# OPPO-to-Apple Portrait Blur-Strength Bracketing

Date: 2026-07-13

> Status: Pass D is superseded by the controlled 1x/2x/3x aperture-series and
> 16 Pro 5x evidence below. Apple uses distinct lens-coupled REND/calibration
> profiles; the product path no longer applies a focal-dependent renderGain.

## Outcome

Real Photos editing has now bracketed the missing render scale:

| Pass | Rank conversion | Apple auxiliary calibration | REND aperture | Device result |
|---|---|---|---|---|
| A | fixed `2.9/255` | physical OPPO-derived | fixed 1.4 | Much too strong; f/16 still heavily blurred |
| B | OPPO header scale | physical OPPO-derived | OPPO XMP value | Reduced, but still too strong; 230mm has a large f/16 jump |
| C | OPPO header scale | canonical donor pair | typed `0x012f` synchronized | Too weak; visible synthetic blur remains small even at f/1.4 |
| D | OPPO header scale × continuous geometric-midpoint gain | canonical donor pair | typed `0x012f` synchronized | Superseded; mixed one 24mm profile with all focal lengths |
| E | OPPO header scale | matched Apple 1x/2x/3x/5x profile with unbounded reference-crop extrapolation | profile endpoints unchanged | Consumed, but incorrectly fabricates a 10x Apple auxiliary chart from the 5x profile |
| F | OPPO header scale | nearest Apple profile, crop capped at its measured range | profile endpoints unchanged | Correct 10x architecture; device blur-strength validation pending |

Pass C proves that canonicalizing the private Apple render domain removes the
double amplification, but removes too much. It is a lower bound, not the final
product setting. Pass B is the upper bound.

## Facts retained

- `deltaSourceDisparity = deltaRank * rear.depth.headerScale` remains supported
  by the 80-file source corpus and the 230mm inverse-distance correlation.
- Actual OPPO physical/equivalent focal length and digital zoom remain primary
  EXIF capture identity.
- `depthBlurEffect:SimulatedAperture` correctly initializes the Photos control.
- Typed REND float record `0x012f` is the profile minimum-aperture endpoint,
  not the current edited aperture. In controlled f/1.4...f/16 series it stays
  at 1.4 while the public/AAE aperture changes.
- Physical OPPO calibration cannot be mixed directly with a fixed 24mm donor
  REND graph. Canonical donor calibration alone is also insufficient.

## No fabricated Apple 10x profile

Apple does not expose a 230mm/10x portrait REND and calibration profile. Pass E
selected the real 5x/120mm profile but then continued shrinking its reference
dimensions by `120/230`, producing a synthetic `2104x1576` chart with
`PixelSize=0.000584348mm`. That is still an invented 9.6x Apple renderer even
though its REND bytes came from a real 5x capture.

Pass F separates capture identity from renderer identity:

- primary EXIF remains OPPO 34.8mm / 230mm;
- Apple auxiliary calibration and REND saturate at the validated 5x/120mm
  chart (`4032x3024`, `PixelSize=0.001120000mm`);
- OPPO rank deltas continue to use only the per-capture header scale;
- OPPO config distance maps to Apple `AFMeasuredDepth`;
- future scene-strength fitting changes disparity gauge/REND scene controls,
  never auxiliary focal length beyond the available Apple profile.

This means 10x does not require a nonexistent Apple 10x donor. It is projected
into the Apple 5x rendering domain. The remaining task is to fit the 5x-domain
scene response from OPPO's focus-selected disparity, config distance,
foreground scale and aperture/blur-strength curve. A scalar or REND fit must
be bounded and profile-local; equivalent focal length is no longer an input
after the 5x saturation point.

## Remaining model

Keep source geometry and Apple rendering scale as separate domains:

```text
sourceDelta = deltaRank * headerScale
appleRenderDelta = sourceDelta * renderGain
```

`renderGain` is a useful Apple-domain probe variable. It must be greater than
the canonical Pass-C value of 1 and smaller than the effective
physical-calibration ratio that produced Pass B. New X9 Ultra native evidence
shows that it is not, by itself, a complete OPPO behavior model.

For the current same-file tests:

- 139mm physical/canonical effective-fx ratio is about `5.6`, so the first
  useful gain sweep is between `1x` and `5.6x`;
- 230mm ratio is about `9.5`, so the useful sweep is between `1x` and `9.5x`.

A geometric-midpoint probe (`~2.4x` at 139mm, `~3.1x` at 230mm) is preferable
to another endpoint guess. Pass D now computes that value continuously from
the two calibration endpoints rather than assigning a gain by focal-length
bucket:

```text
endpointRatio = sourceEffectiveDepthFx / canonicalEffectiveRenderFx
renderGain = sqrt(max(1, endpointRatio))
appleRenderDelta = deltaRank * headerScale * renderGain
```

The source header fx is continuous at intermediate digital zoom values, so
41/47/59/90/98/125/164/206mm captures do not require interpolation between
optical switch points.

## Pass-D 230mm probe

`IMG20260506112827` is a real 230mm portrait captured at f/10 with config
distance 442. Its source fields are:

- depth plane `1024x768`, header fx `6892.707`, effective source fx
  `27570.828` at base-image resolution;
- header rank scale `0.0050056945` and rank p99-p01 span `234`;
- source disparity p99-p01 span `1.17133`;
- canonical effective render fx `2860.379`, endpoint ratio `9.4883x`, and
  Pass-D `renderGain = 3.0803x`.

The emitted Float16 disparity measures p01 `1.5703`, p50 `4.3125`, p99
`5.1797`, and p99-p01 `3.6094`. The Pass-C 1x control measures p99-p01
`1.1699`. Primary HEVC payload, active gain-map payload and OPPO private tail
remain byte-identical; only the intended portrait auxiliary graph changes.

The candidate also uses the current OPPO M6 matte path: portrait and pet
planes determine subject topology, hair is merged into PEM and emitted as an
independent semantic hair matte, and RGB guidance refines only the narrow
matte boundary. The original rank-derived disparity is still written.

## Focal-length re-analysis

The 80-file corpus does not support applying a second linear focal multiplier
to disparity. Median source and Pass-D p99-p01 spans are:

| Equivalent mm | Samples | Median source span | Median Pass-D span | Median gain |
|---:|---:|---:|---:|---:|
| 23 | 9 | 1.050 | 1.051 | 1.001 |
| 47 | 12 | 1.096 | 1.566 | 1.428 |
| 70 | 23 | 0.721 | 1.209 | 1.676 |
| 139 | 5 | 0.829 | 1.969 | 2.375 |
| 230 | 19 | 0.265 | 0.817 | 3.080 |

Although gain and equivalent focal length are strongly related because they
describe the same continuous projection geometry, the final Pass-D span has
only weak corpus-wide correlation with focal length (`r ~= 0.24`). At 230mm,
source span instead has strong correlation with inverse config distance
(`r ~= 0.79`), which Pass D deliberately preserves. Near 230mm captures have
Pass-D spans around `3.04...3.92`; the longer-distance mode is only
`0.48...0.86`. Therefore `IMG20260506112827` is intentionally a high-amplitude
near probe, not a generic 230mm constant.

## OPPO blur curve

`rear.depth.config` already provides 22 aperture/blurValue pairs. These should
be treated as curve-shape evidence, not substituted directly for Apple
disparity. A normalized source curve can be formed as:

```text
curve(f) = (blurValue(f) - blurValue(16))
           / (blurValue(1.4) - blurValue(16))
```

The final mapping needs at least:

1. a per-lens or smoothly focal-dependent amplitude (`renderGain`);
2. the normalized OPPO curve to shape f/1.4...f/16 response;
3. zoom-region and focus-distance/scene terms;
4. an explicit low-blur endpoint rather than assuming displayed f/16 alone
   disables rendering;
5. the canonical Apple calibration/REND domain for Photos interoperability.

Firmware reverse engineering explains why. OPPO's native renderer calculates
focus depth from face/portrait/pet/point/scene inputs, then applies separate
foreground/background nonlinear CoC functions with different 1-2x, 2-3x,
3-6x, and 6-10x branches. At displayed f/16 it internally uses f/40 below
about 6x or f/20 at 6-10x. Apple Photos does not inherit that rule merely from
the OPPO `fNumber` value. See
`oppo-x9-ultra-portrait-depth-consumption-20260713.md`.

Gallery also sends both `f_aperture` and the capture table's paired
`blur_strength`. Its bundled ArcSoft fallback JNI clamps `blur_strength` to
`0...200` and passes it to the processing engine alongside aperture and focus
geometry. The exact X9 Ultra OPLUS-backend formula for this second control is
still unresolved, but treating the table as aperture labels alone is already
disproved.

## Acceptance matrix

Use the same images at f/1.4, source aperture, and f/16:

- 23mm and 70mm optical controls;
- 139mm exact comparison image;
- 230mm near and medium-distance samples.

The target is visible but natural f/1.4 separation, a plausible source-aperture
default, and minimal synthetic jump at f/16. Portrait controls, Focus, gain
map, disparity, PEM, orientation, and payload preservation remain hard gates.

Pass D is not approved for merge until Photos device testing establishes that
its f/16 endpoint no longer jumps while f/1.4 remains visibly useful. PR #7
remains draft.

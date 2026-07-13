# OPPO-Derived Apple Camera Calibration Plan

Date: 2026-07-13

## Goal

Replace the fixed iPhone disparity calibration currently written by
`--apple-portrait` with a per-capture model derived from OPPO EXIF and the
stored `src.image` geometry.

Also replace the fixed Apple simulated aperture with the OPPO portrait edit
f-number while preserving the device-validated rendering-parameter graph.

## Model

1. Read physical focal length, 35mm-equivalent focal length, digital zoom
   ratio, and lens model from the OPPO primary EXIF.
2. Resolve the optical anchor from the lens model (`23mm`, `70mm`, etc.), with
   `equivalent / digitalZoomRatio` as a fallback.
3. Estimate the anchor focal length in pixels from the reference-image
   diagonal and the 35mm-frame diagonal.
4. Match Apple's observed crop representation: keep the anchor focal length in
   the intrinsic matrix while shrinking the reference dimensions by the
   continuous digital zoom ratio.
5. Derive effective pixel size from physical focal length and the anchor pixel
   focal length, then scale it by the zoom ratio.
6. Until real OPPO lens distortion tables are available, describe the
   registered OPPO source/depth pair as rectified with zero forward/inverse
   distortion instead of copying an unrelated iPhone lens profile.
7. Read the portrait f-number from `rear.depth.config` v4 offset 292, fall back
   to EXIF `FNumber`, and reserve `f/1.4` for missing/invalid metadata only.

## Validation

- Swift CLI type-checks with warnings as errors.
- A 7.73mm/23mm-anchor portrait and a 20.1mm/70mm-anchor portrait convert.
- ExifTool reports different, depth-header-derived calibration matrices for
  physical lenses and continuous zoom ratios.
- ImageIO still exposes ISO gain map, disparity, Portrait Effects Matte, and
  Focus XMP.
- ExifTool reports the OPPO portrait f-number as `SimulatedAperture`.
- Portrait/landscape selection follows the outer image's displayed aspect and
  the `src.image` EXIF orientation; it must not depend on exact swapped pixel
  dimensions.
- Final iPhone import remains the device acceptance gate.

## Blur-scale regression correction

Device validation showed that changing camera calibration while retaining the
fixed `1.4...4.3` disparity interval makes Photos blur increasingly strongly
at longer focal lengths. The OPPO uint8 ranks are relative depth, not already
calibrated Apple disparity; their numeric span must be converted together with
the focal calibration.

The corrective model is:

1. Parse the 768-byte decoded `rear.depth` header instead of discarding it.
2. Read the per-capture depth dimensions and effective focal length at offset
   `0x1c`. This value already varies continuously at digital zoom focal lengths
   such as 41, 47, 59, 90, 98, 125, 139, 164, and 206mm.
3. Retain header focal length and baseline as source-depth diagnostics, but do
   not feed their physical scale directly into Apple's private renderer.
4. Keep the Apple auxiliary calibration paired with the donor `REND` graph.
   The real OPPO lens remains in primary EXIF; the auxiliary calibration is an
   interoperability render profile, not a replacement capture identity.
5. Convert uint8 rank differences with the per-capture Float32 scale at header
   offset `0x18`: `deltaDisparity = deltaRank * headerScale`. The previous
   fixed step was `2.9/255 = 0.01137`; 80 real OPPO portraits instead expose
   steps in approximately the `0.0011...0.0051` range.
6. Keep the OPPO f-number mapping independent of disparity normalization.

The initially inspected 230mm sample `IMG20260713083446` was not an OPPO
portrait asset, but the expanded original corpus contains 19 real 230mm
portrait captures with depth. Their depth-header focal length is consistently
`6892.71` pixels at `1024x768`. Near captures use a rank step around `0.0050`,
while the longer-distance mode uses a step around `0.00117`; therefore neither
focal length nor subject distance alone is sufficient to derive disparity.

Additional acceptance checks:

- 23, 47, 70, 139, and 206mm portrait samples log continuous header-derived
  effective focal lengths and decreasing disparity spans.
- emitted disparity differences equal rank differences multiplied by the
  per-capture header scale.
- Apple auxiliary calibration and `REND` remain the canonical matched pair;
  70/139/230mm source focal lengths must not amplify blur a second time.
- iPhone Photos must show substantially reduced blur at f/16 while retaining
  the mapped initial aperture and portrait controls.

## Device bracketing result

- Physical OPPO auxiliary calibration plus header-scaled disparity remains too
  strong, particularly at 230mm f/16.
- Canonical donor calibration plus the same header-scaled disparity is too
  weak, with little visible effect even at f/1.4.
- Synchronizing typed REND aperture record `0x012f` does not restore strength.

Therefore the next implementation must introduce an explicit render-domain
gain between those bounds instead of selecting either calibration endpoint.
See `docs/research/oppo-apple-portrait-blur-strength-bracketing-20260713.md`.

## Pass D status

The first continuous-gain implementation is ready for device testing:

```text
renderGain = sqrt(sourceEffectiveDepthFx / canonicalEffectiveRenderFx)
```

It retains the canonical Apple calibration/REND pair and multiplies only the
rank-to-disparity delta. It also incorporates the current M6 OPPO matte path,
so the comparison is not confounded by the older Vision-only PEM fallback.

For `IMG20260506112827` (230mm, f/10, config distance 442), the endpoint ratio
is `9.4883x` and Pass-D gain is `3.0803x`. Offline checks confirm Orientation 6,
OPPO capture EXIF, source-primary and gain-map payload preservation, disparity,
PEM, semantic hair matte, and Focus metadata. Final Photos behavior at f/1.4,
f/10, and f/16 remains the acceptance gate; no merge is authorized yet.

## Corpus batch and product-mode status

The Swift batch command now accepts `--apple-portrait`, filters ordinary HEIC
inputs, validates Apple portrait outputs for resume/skip-existing, and includes
portrait mode in its checkpoint configuration hash. A clean 80-original batch
completed with gain map, disparity and PEM visible for every output; semantic
hair is present for the 44 inputs with a usable OPPO hair plane.

Apple portrait conversion and OPPO-compatible preservation are now mutually
exclusive. Apple mode forces OPPO compatibility off and omits the redundant
large OPPO portrait tail. Both single and batch parsers reject an explicit
attempt to enable the two output modes together.

This closes batch operability, not fidelity. The 80-file audit still identifies
unmapped config focus/blur-curve data, crop/mesh registration and 4.10-7.31 MiB
of later semantic/confidence/YUV buffers per decoded depth package. Those gaps
and the pending device matrix keep this plan active.

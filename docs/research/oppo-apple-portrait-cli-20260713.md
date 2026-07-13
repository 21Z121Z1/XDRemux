# OPPO to Apple Portrait CLI Integration

Date: 2026-07-13

## Product behavior

The production Swift CLI now exposes one opt-in switch:

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --apple-portrait \
  --input INPUT.heic \
  --output OUTPUT.heic
```

Without the switch, XDRemux keeps the normal gain-map conversion path and
reattaches the original OPPO portrait private tail byte-for-byte. It does not
generate Apple portrait resources. With the switch, conversion requires both:

- the OPPO portrait bit (`65536`) in the numeric `UserComment` value;
- a private tail containing `rear.depth`.

The CLI then extracts `src.image`, `local.uhdr.gainmap.info`, and `rear.depth`
from the manifest. `rear.depth` is zstd-decoded; the first uint8 rank plane is
derived at one-quarter of the stored base width and height.

## Apple portrait output

The Apple output contains:

- base and ISO gain map from the paired JPEGs in `src.image`;
- relative Float16 disparity derived from OPPO uint8 depth ranks;
- Vision accurate person segmentation as Portrait Effects Matte;
- Vision face detection ranked by attention saliency, with attention-centroid
  fallback;
- geometry-aware automatic orientation selection and displayed-to-stored
  Focus-coordinate mapping for EXIF orientations 1-8;
- OPPO capture EXIF/GPS plus the validated 57-field Apple interoperability
  MakerNote profile;
- portrait rendering parameters, the OPPO-matched simulated aperture, Portrait
  Lighting strength 0.5, and `CustomRendered=9`.

The embedded Apple compatibility data contains metadata only, not reference
pixels.

## OPPO-derived camera calibration

The disparity metadata no longer copies a fixed iPhone calibration profile.
For each OPPO capture, XDRemux now:

- reads physical `FocalLength`, 35mm-equivalent focal length,
  `DigitalZoomRatio`, and `LensModel` from EXIF;
- resolves the active optical anchor from the lens model (for the validated
  Find X9 Ultra samples, `23mm` or `70mm`);
- estimates the anchor pixel focal length from the `src.image` diagonal and
  the 35mm-frame diagonal;
- keeps that anchor focal length while shrinking the intrinsic reference
  dimensions by the continuous digital zoom ratio, matching the representation
  observed in iPhone portrait captures;
- derives effective PixelSize from physical focal length and the same crop
  ratio;
- writes centered principal/distortion points and zero forward/inverse
  distortion under the assumption that the OPPO `src.image` and decoded depth
  plane are already registered.

This is a geometrically consistent pinhole approximation, not a substitute for
factory OPPO distortion tables. Camera2 lens intrinsic/distortion metadata or
per-lens checkerboard calibration remains the path to exact edge geometry.

## OPPO-derived simulated aperture

The aperture shown by the portrait editor is not the physical capture
aperture. OPPO stores this bokeh setting as `fNumber` in `RearDepthStruct v4`
at byte offset 292 of `rear.depth.config`. Across the validated portrait
samples, the decoded values (`f/3.5`, `f/4.5`, `f/5.0`, `f/5.6`, and `f/6.3`)
also match EXIF `FNumber`.

XDRemux now writes Apple's `depthBlurEffect:SimulatedAperture` using this
precedence:

1. `rear.depth.config` v4 `fNumber`;
2. EXIF `FNumber` from the outer image or `src.image`;
3. `f/1.4` only as the compatibility fallback when neither source is valid.

The already device-consumable Apple `RenderingParameters` template remains
unchanged. It is a 1,352-byte `REND` parameter graph containing several values
equal to 1.4 with different semantics; replacing matching float bytes without
a field-level schema would corrupt unrelated rendering controls.

## Encoding boundary

The real `src.image` base and gain map are encoded once into the first ISO
gain-map assembly. Auxiliary authoring uses a blank scaffold. The final remux
replaces the scaffold base/gain tile payloads with the first-assembly payloads
and patches only `iloc` offsets/lengths and `mdat` size.

## Real sample evidence

Input: `IMG20260606175915.heic`.

The integrated no-field extraction path completed successfully before the
final metadata-only calibration/color-space refinements. The resulting
container passed the repository ISO parser and reported:

- primary `4096x3072` grid with 48 HEVC tiles;
- ISO gain-map `2048x1536` grid with 12 HEVC tiles;
- disparity `1024x768`, `hdis`, relative/high/filtered, range
  `1.4003906...4.3007812`;
- Portrait Effects Matte auxiliary item and XMP;
- one Focus region;
- OPPO Make/Model/ISO;
- 57 Apple MakerNote fields, version 17, image capture type 12, feature flags 1;
- base 48/48 and gain 12/12 tile payloads byte-identical to the first assembly.

The integrated output passed a real iPhone Photos import and portrait-consumer
check before the OPPO-derived calibration change. Without `--apple-portrait`,
the current mainline conversion path preserves the complete OPPO/QTI camera
tail by default.

## Calibration validation

Two real Find X9 Ultra portrait captures were converted after this change:

- `IMG20260713083415`: physical `7.73mm`, optical anchor `23mm`, zoom
  `2.022x`, reference `2018x1516`, `fx=fy=2712.374`, PixelSize
  `0.001409447mm`;
- `IMG20260713001840`: physical `20.1mm`, optical anchor `70mm`, zoom
  `1.9824x`, reference `2066x1550`, `fx=fy=8283.523`, PixelSize
  `0.001224023mm`.

ExifTool recovered those exact per-capture values. AVDepthData returned a
non-null `cameraCalibrationData` for both files, while ImageIO still exposed
the ISO gain map, disparity, Portrait Effects Matte, and Focus XMP.

The first calibration test exposed an independent orientation regression: the
outer OPPO portrait (`3064x4592`, Orientation 1) and landscape-stored
`src.image` (`4080x3064`, Orientation 6) did not have exactly swapped pixel
dimensions, so the previous equality check incorrectly emitted Orientation 1.
Orientation selection now compares the outer image's displayed aspect with
each `src.image` orientation candidate. Regenerated 23mm, 47mm, and 70mm
samples all report Orientation 6 while retaining ISO gain map, disparity, PEM,
and Focus.

The regenerated OPPO-calibrated, orientation-corrected output subsequently
passed real iPhone Photos portrait consumption. This device result closes the
acceptance gate for the calibration/orientation change.

## Simulated-aperture validation

Three regenerated captures exercise both physical-lens anchors and continuous
zoom. The CLI decoded `rear.depth.config` v4 and ExifTool recovered the same
Apple `SimulatedAperture` values:

- `IMG20260713083412`: `f/3.5`;
- `IMG20260713083415`: `f/4.5`;
- `IMG20260713083419`: `f/5.0`.

All three remain Orientation 6. ImageIO exposes the ISO gain map, disparity,
and Portrait Effects Matte for each output; AVDepthData reports non-null camera
calibration, and the Focus XMP remains present. The remaining device-level
check is that Photos initializes its portrait aperture control to the matched
OPPO value rather than merely preserving the metadata tag.

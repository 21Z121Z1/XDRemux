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

The same switch is available in batch mode; non-portrait HEIC files are
filtered before conversion:

```bash
swift xdremux/swift-cli/XDRemux.swift batch \
  --apple-portrait \
  --input-dir INPUT_DIR \
  --output-dir OUTPUT_DIR
```

Without the switch, XDRemux keeps the normal gain-map conversion path and
reattaches the original OPPO portrait private tail byte-for-byte. It does not
generate Apple portrait resources. With the switch, the strong match is the
OPPO portrait bit (`65536`) plus `rear.depth + rear.depth.config + src.image`.
An explicit conversion also accepts the same complete resource set when the
UserComment route bit has been lost, and emits a recovery warning.

The CLI extracts `src.image`, gain metadata, and `rear.depth` from the
manifest. When private `local.uhdr.gainmap.info` is missing but the outer HEIC
already contains an ISO gain map, it reconstructs the same 20-float model from
`HDRToneMap` rather than inventing defaults. `rear.depth` is zstd-decoded; the
first uint8 rank plane begins after its 768-byte header.

Apple portrait and OPPO-compatible preservation are mutually exclusive
product modes. Apple output omits the large redundant OPPO portrait tail;
explicitly enabling both modes is a parse-time error for `convert` and
`batch`.

## Apple portrait output

The Apple output contains:

- base and ISO gain map from the paired JPEGs in `src.image`;
- relative Float16 disparity derived from OPPO uint8 depth ranks;
- OPPO portrait/pet topology as Portrait Effects Matte, with Vision accurate
  person segmentation only when those subject planes are empty;
- OPPO hair merged into PEM and emitted as an independent semantic hair matte;
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

## Lens-profile camera calibration

The disparity metadata no longer maps every rank plane into one fixed interval
or couples every focal length to one 24mm Apple renderer profile.
For each OPPO capture, XDRemux now:

- reads physical `FocalLength`, 35mm-equivalent focal length,
  `DigitalZoomRatio`, and `LensModel` from EXIF;
- resolves the active optical anchor from the lens model (for the validated
  Find X9 Ultra samples, `23mm` or `70mm`);
- reads the per-capture effective depth focal length and rank-to-disparity
  scale from the decoded 768-byte `rear.depth` header;
- preserves the measured header focal length/baseline as source-depth
  diagnostics but uses header rank scale, not physical focal length, to convert
  rank differences;
- selects a matched Apple 1x-main, 2x/Fusion, 3x-tele, or 5x-tetraprism
  `REND`/calibration pair from controlled real captures;
- scales that profile's reference crop, principal point, distortion center,
  and PixelSize by `profileAnchorEquivalent / sourceEquivalent` only inside
  the profile's measured range, while keeping the profile intrinsic fx fixed.
  This matches 2x and 3x Apple continuous-zoom samples with median
  reference-width error 0 and p95 error below 0.6%; beyond the measured range,
  the private Apple chart saturates instead of fabricating a longer profile;
- converts rank deltas only with the OPPO header scale, with no focal-dependent
  `renderGain`;
- retains the real OPPO physical/equivalent focal length and digital zoom in
  primary EXIF.

Controlled 48mm and 77mm edits show similar normalized f/1.4-to-f/16 response
despite a 1.6x intrinsic-fx change, confirming that Apple lens profiles perform
the cross-focal normalization. The 16 Pro 120mm profile provides the terminal
Apple render domain for OPPO 230mm. Primary EXIF still records the real 230mm
capture, but the private auxiliary chart remains fixed at 5x rather than
extrapolating a nonexistent Apple 10x profile.

An 80-file original portrait corpus, including 19 real 230mm captures, showed
that non-optical focal lengths already carry continuous header focal lengths.
It also showed that the per-rank disparity scale changes with capture/depth
mode, especially at 230mm. See
`docs/research/oppo-depth-header-disparity-scale-20260713.md`.

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
unchanged except for selecting the matching lens profile. It is a 1,352-byte
`REND` parameter graph containing several values equal to 1.4 with different
semantics, so matching float bytes must not be replaced globally. Controlled
1x/2x/3x captures with different initial editing apertures show that typed
record `0x012f` stays at the profile's minimum-aperture endpoint rather than
tracking the current simulated aperture. XDRemux therefore writes the OPPO
value to `depthBlurEffect:SimulatedAperture` and does not patch `0x012f`.

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
- historical pre-correction disparity `1024x768`, `hdis`,
  relative/high/filtered, range `1.4003906...4.3007812`;
- Portrait Effects Matte auxiliary item and XMP;
- one Focus region;
- OPPO Make/Model/ISO;
- 57 Apple MakerNote fields, version 17, image capture type 12, feature flags 1;
- base 48/48 and gain 12/12 tile payloads byte-identical to the first assembly.

The integrated output passed a real iPhone Photos import and portrait-consumer
check before the OPPO-derived calibration change. Without `--apple-portrait`,
the current mainline conversion path preserves the complete OPPO/QTI camera
tail by default.

The latest clean batch contains 80 deduplicated camera originals. Offline
ImageIO inspection reports gain map, disparity and PEM for all 80, plus
semantic hair for the 44 originals with a nonzero hair plane. The source set is
483.46 MiB and Apple outputs total 98.76 MiB because the converted graph does
not retain a second private OPPO re-edit package. See
`docs/research/oppo-apple-portrait-information-coverage-20260713.md` for the
mapping and remaining gaps.

## Calibration validation

The first EXIF-only calibration pass converted two real Find X9 Ultra portrait
captures:

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
passed real iPhone Photos portrait consumption, but device editing exposed a
blur-scale regression: f/16 remained far too blurred. The cause was the fixed
`2.9/255` rank step combined with variable focal calibration. The current pass
replaces both the EXIF-estimated focal scale and fixed disparity interval with
the per-capture `rear.depth` header values documented above. Final device
editing remains the acceptance gate.

A second device pass using header-scaled disparity but physical OPPO auxiliary
calibration still showed excessive f/16 blur, especially at 230mm. This
isolated the remaining failure to the mismatched physical calibration plus
fixed donor `REND`. A historical probe also synchronized `0x012f`, but the
later controlled series proved that it is not the current-aperture field; the
product path now leaves the selected Apple profile record unchanged.

That canonical-pair pass established the opposite bound: synthetic blur became
too weak and remained subtle even when Photos was set to f/1.4. The result
proves that the final solution needs an explicit render gain between canonical
and physical endpoints. Record `0x012f` is useful for aperture consistency but
is not the missing strength control. Detailed device conclusions and the next
gain sweep are recorded in
`docs/research/oppo-apple-portrait-blur-strength-bracketing-20260713.md`.

## Simulated-aperture validation

Three regenerated captures exercise both physical-lens anchors and continuous
zoom. The CLI decoded `rear.depth.config` v4 and ExifTool recovered the same
Apple `SimulatedAperture` values:

- `IMG20260713083412`: `f/3.5`;
- `IMG20260713083415`: `f/4.5`;
- `IMG20260713083419`: `f/5.0`.

All three remain Orientation 6. ImageIO exposes the ISO gain map, disparity,
and Portrait Effects Matte for each output; AVDepthData reports non-null camera
calibration, and the Focus XMP remains present. Photos was subsequently
verified to initialize its portrait aperture control to the mapped OPPO value.
The remaining work concerns calibrating render gain and aperture-curve shape
across focal lengths and distances, not aperture-tag recognition.

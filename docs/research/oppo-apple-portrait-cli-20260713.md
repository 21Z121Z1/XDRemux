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
- portrait rendering parameters, simulated aperture 1.4, Portrait Lighting
  strength 0.5, and `CustomRendered=9`.

The embedded Apple compatibility data contains metadata only, not reference
pixels.

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

The production Swift file type-checks after the final orientation,
color-space, and calibration metadata refinements. A fresh iOS import remains
the final acceptance gate for the integrated output.

The ordinary no-switch path was confirmed to run before tail preservation was
added. The new byte-for-byte tail reattachment code type-checks; its real-file
rerun was blocked by the current local execution quota and remains an explicit
targeted follow-up.

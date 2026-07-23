# Swift CLI

This directory contains the compatibility entry point for the Swift
command-line converter. The implementation now lives in the root Swift Package
under `Sources/`.

The preferred command from the repository root is:

```bash
swift run xdremux convert --input IMG_001.heic
```

Existing scripts may continue to use the legacy entry point. It locates the
repository root and forwards all arguments to the same `xdremux` executable:

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

## Product modes

- no switch: standard ISO output, non-HDR metadata-tail preservation, and up to
  HEVC RExt 4:4:4 Gain Map when the source channel structure permits;
- `--oppo-compatible`: OPPO Gallery-compatible Main Still Picture 4:2:0 with
  the complete OPPO private tail;
- `--apple-photographic-styles`: donor-free, current-input Apple Photographic
  Styles payload and HEIF auxiliary graph;
- `--apple-portrait`: OPPO portrait resources converted to the Apple portrait
  graph, without retaining a second large OPPO portrait tail.

The two Apple switches are independent and can share one final HEIC.
`--oppo-compatible` is mutually exclusive with either Apple switch. Existing
4:2:0 sources are never promoted to 4:4:4 because missing chroma cannot be
recovered.

## Apple portrait conversion

Apple features are opt-in:

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic

swift run xdremux batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/

swift run xdremux batch \
  --apple-photographic-styles \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/
```

`--apple-portrait` requires the recoverable `rear.depth + rear.depth.config +
src.image` resource set. The UserComment portrait bit is the strong route; an
explicit conversion may recover a missing bit and emits a warning. XDRemux
uses `src.image` storage coordinates for the saved focus point, parses the v1–v4
config and saved rank quantizer as typed data, and reconstructs the producer's
rank-to-float-depth domain. `CalFocusDepthEngine` dispatch follows saved
near-object, scene-class, and focus-ROI state; its exact branch evidence is
reported separately from remaining face/pet ROI fallbacks. Vision remains the
high-resolution semantic producer. OPPO person/pet and hair
planes are edge-guided topology priors: person supplementation must overlap
Vision topology and hair supplementation is gated by the final person matte.
Skin, teeth, and glasses remain Vision-only. Vision face attention is used only
when the OPPO package cannot supply a valid focus scene.

Portrait-only batch reports non-portrait inputs as unavailable. Combined batch
continues those inputs as styles-only. OPPO-compatible preservation remains
mutually exclusive with both Apple features. A successful combined Portrait +
Styles conversion shares the Portrait Vision request batch and its seven matte
results with the Styles payload builder instead of launching the same semantic
analysis twice.

Styles-only semantic persistence follows native role tiers: sky-only without a
credible person, and PEM+skin+sky with one. Portrait writes the complete
PEM+skin+hair+teeth+glasses family atomically; combined output adds sky. Sparse
valid results are retained, but unavailable private Vision SPI is never
silently represented by a fake mask.

The complete `src.image` blob is the sole Base/Gain source for both HEIC and
JPEG portrait inputs. ImageIO converts it directly to HEIC with
`kCGImageDestinationPreserveGainMap`; RGB `444f` and grayscale `L008` Gain Maps
retain their channel structure, while unreadable or 4:2:0 sources fail closed.
The CLI requires `zstd` on `PATH` to decode OPPO `rear.depth`.

The decoded `rear.depth` header supplies min/max/exponent quantization and the
relative disparity scale, avoiding the old fixed interval and linear-rank
assumption. The Apple auxiliary graph is selected from real 1x, 2x/Fusion, 3x
tele, or 5x tetraprism calibration domains. Each stored REND profile contains
only 153 invariant records. Producer-confirmed XHLRB scene records
`0x0190...0x0199` and `0x01c2...0x01c5` are appended per input, sorted with the
firmware serializer, and validated by a byte-stable parser. No complete donor
REND remains in the product source. Within a profile, reference dimensions,
principal point, distortion center, and PixelSize follow Apple's observed
continuous-crop representation while intrinsic fx remains fixed. Disparity is
not multiplied by focal length a second time. Real OPPO lens and zoom identity
stays in primary EXIF; 230mm input saturates the Apple auxiliary at its
validated physical profile instead of inventing a 10x profile.

Every successful portrait write produces a sibling
`<output>.portrait-manifest.json`. It includes the two firmware builds, complete
typed config/header, focus branch and ROI, OPPO internal-disparity feature,
Apple relative-disparity range, blur curve, static/dynamic REND records,
native-generator lookup result, warnings, and evidence classes. The current
XHLRB output scaler and its `ISOSpeedRating × ExposureTime`, clipped-pixel and
`GainMapHeadroom` inputs come from the iOS 26.5 producer. `0x01c5` must equal
the current ISO Gain Map headroom. The remaining exposure/clipped-pixel
activation thresholds are `controlled_corpus_fit`; the ObjC wrapper is absent
on macOS even though the recovered iOS Metal kernel itself is runtime
compatible.

Without `--debug-dir`, Photographic Styles evidence, semantic matte dumps, and
payload encoder inputs use temporary storage and are removed when conversion
exits. With `--debug-dir`, the complete Styles evidence is retained under
`<debug-dir>/<input-stem>/photographic-styles/runs/<run-identifier>/`, with
`photographic-styles/latest.json` pointing to the newest run.

Validate an output independently:

```bash
swift run xdremux validate-portrait \
  --input IMG_001_apple_portrait.heic \
  --json IMG_001_apple_portrait.validation.json

swift run xdremux portrait-self-test
```

The validator requires all portrait auxiliaries, Focus XMP, REND dynamic
invariants, the recovered XHLRB scaler relation, `01c5`/GainMapHeadroom
identity, byte-stable round-trip, and zero matches from the known-donor payload
scanner.

Cross-focal blur matching remains experimental until the source-derived
outputs pass the full Photos f/1.4/source/f/16, refocus, and save/reopen matrix.
The payload-preserving writer now maps each source base/gain item's `hvcC`
property onto the portrait scaffold, replaces variable-length codec boxes, and
updates ancestor box sizes plus construction-method-0 `iloc` offsets. This
closes the observed 111/112-byte 230mm codec-graph mismatch while preserving
the source bitstreams exactly.

One combined 139mm/3x-crop output has passed macOS Photos at source f/6.3,
f/1.4, f/16, foreground/background refocus, and save/unload/reopen, with both
Portrait and Photographic Styles still present. This does not close the
multi-profile or physical-iOS acceptance matrix.

One 230mm/5x-saturated portrait-only output also passed macOS Photos at source
f/10, f/1.4 and f/16, background/subject refocus, and saved f/1.4 after leaving
and reopening the photo. The Apple auxiliary remains the validated 120mm
physical profile while primary EXIF remains 230mm. Photos exposes a disabled
low-resolution/unsupported-format badge for both tested candidates, so the
full device matrix remains open.

OPPO HDR JPEG portrait inputs are supported when they carry an ImageIO-readable
Gain Map in the complete `rear.depth + rear.depth.config + src.image` bundle.
The gain parameters come from the outer private graph when present, or from the
same complete `src.image` metadata for JPEG inputs. ImageIO may expose one
shared `HDRToneMap:ChannelMetadata` record when all raster channels use the same
curve; the bridge maps that record equally across the RGB parameter slots.
Batch selection remains explicit through `--glob '*.jpg'`. JPEG input is
accepted only when `--apple-portrait` is enabled, optionally together with
Photographic Styles; all other product modes keep the existing HEIC-only input
contract.

The Apple simulated aperture is taken from the OPPO portrait edit state in
`rear.depth.config` when available, then from EXIF `FNumber`; `f/1.4` is only a
last-resort compatibility fallback. The resolved value is written as
`depthBlurEffect:SimulatedAperture`.

Without `--apple-portrait`, XDRemux uses its normal gain-map conversion path
and preserves the OPPO portrait tail while filtering private HDR tail entries.
It does not synthesize Apple depth, matte, Focus, or portrait metadata.

Package layout:

- `Sources/XDRemuxCore/` owns platform-neutral conversion and container logic.
- `Sources/XDRemuxAppleFeatures/` owns Apple-only conversion features.
- `Sources/XDRemuxCLI/` owns parsing, commands, and terminal output.
- `Tests/XDRemux*Tests/` contains the SwiftPM regression tests.

Do not place macOS app project files here; app shells belong under `apps/macos/`.

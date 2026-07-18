# Swift CLI

This directory contains the compatibility entry point for the Swift
command-line converter. The implementation now lives in the root Swift Package
under `Sources/`.

The preferred command from the repository root is:

```bash
swift run xdremux convert --input IMG_001.heic
```

Run `swift build` once before using an Apple feature from a source checkout.
That unscoped build produces the semantic, HEVC encoder, and style-validation
helper executables. XDRemux locates only prebuilt helpers beside the CLI or in
an App bundle; it never searches for helper source or invokes a compiler at
runtime.

The helper protocols keep versioned machine data on stdout and diagnostics on
stderr. Current schemas are `xdremux-semantic-helper-v1`,
`xdremux-hevc-encoder-helper-v1`, and
`xdremux-apple-semantic-style-properties-probe-v1`.

Existing scripts may continue to use the legacy entry point. It locates the
repository root and forwards all arguments to the same `xdremux` executable:

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

## Public commands and options

```text
xdremux convert --input <file> [--output <file>] [options]
xdremux batch --input-dir <dir> [--output-dir <dir>] [options]
```

The public product options are:

- `--input`, `--output`, `--input-dir`, `--output-dir`, `--glob`, and `--jobs`;
- `--overwrite` and `--discard-portrait-data`;
- `--oppo-compatible`, `--apple-photographic-styles`, and `--apple-portrait`;
- `--quiet`, `--verbose`, `--debug`, and `--format text|json|jsonl`;
- `--language auto|zh-Hans|en`.

Styles and Portrait may be enabled together. OPPO-compatible output conflicts
with either Apple feature and is rejected before conversion starts.

## Output and localization

Text help is written to stdout. Human-readable progress, warnings, failures,
and summaries are written to stderr. On an interactive stderr terminal,
XDRemux uses one in-place progress region. Pipes, redirection, and CI use plain
line output and never receive ANSI control sequences.

Default batch text does not print one success line per file. `--verbose` adds
per-file completion and skip lines. `--debug` additionally includes internal
configuration, helper activity, temporary paths, and underlying errors.
`--quiet` keeps failures and the final result.

`--format json` emits one document containing an `events` array. `--format jsonl`
emits one object per line. Both formats use `schema_version: 1`; field
names, event names, warning codes, and error codes are stable English
identifiers. Only the human `message` field is localized. For example:

```json
{"schema_version":1,"event":"conversion_failed","error_code":"source_gain_map_missing","input":"IMG_001.heic","message":"The source photo does not contain a usable HDR Gain Map."}
```

Stable failure codes are:

| Code | User-facing category |
| --- | --- |
| `source_not_found` | Input path does not exist |
| `source_not_supported` | Unsupported source photo |
| `source_gain_map_missing` | No usable HDR Gain Map |
| `source_gain_map_corrupt` | Incomplete or damaged Gain Map |
| `portrait_data_unavailable` | Required portrait resources are unavailable |
| `apple_runtime_unavailable` | Required Apple processing service is unavailable |
| `output_not_writable` | Output cannot be created or replaced |
| `output_verification_failed` | Written output failed validation |
| `internal_container_error` | Unsupported internal container condition |
| `invalid_arguments` | Invalid command-line arguments |
| `batch_incomplete` | Batch ended with failures |

Language resolution uses the first available value from `--language`,
`XDREMUX_LANGUAGE`, `Locale.preferredLanguages`, and the English fallback.
Supported language identifiers are `zh-Hans`/`zh-CN` and
`en`/`en-US`/`en-GB`.

Exit codes are:

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Internal container error |
| `2` | Invalid command or arguments |
| `3` | Missing, unsupported, or invalid input |
| `4` | Output or Apple runtime failure |
| `5` | Batch completed with one or more failures |
| `130` | Interrupted with Ctrl+C |

## Batch reruns

Batch output preserves each input's path relative to `--input-dir`, so equal
filenames in different albums remain distinct. Existing outputs are checked
with the current lightweight output validator: valid files are skipped,
invalid files are regenerated, and `--overwrite` always regenerates. A sibling
temporary file is validated before atomic installation. One file failure does
not stop the remaining work, and the final report is written to
`<output-dir>/xdremux-failures.json`.

The old JSONL checkpoint header, config hash, mtime/size signature state, and
resume journal are no longer used.

## Developer executable

Internal conversion controls and validators moved to `xdremux-dev`:

```bash
swift run xdremux-dev convert \
  --input IMG_001.heic \
  --family x7 \
  --input-processing hybrid \
  --oppo-compat auto \
  --oppo-camera-tail preserve \
  --tmap-format imageio \
  --diagnostics-dir diagnostics/
```

`xdremux-dev` also provides `validate-apple`, `validate-portrait`, and
`portrait-self-test`. The public `xdremux` parser rejects all of these internal
options and commands.

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
mutually exclusive with both Apple features.

Styles-only semantic persistence follows native role tiers: sky-only without a
credible person, and PEM+skin+sky with one. Portrait writes the complete
PEM+skin+hair+teeth+glasses family atomically; combined output adds sky. Sparse
valid results are retained, but unavailable private Vision SPI is never
silently represented by a fake mask.

The `src.image` unblurred base is encoded once. For an HDR JPEG portrait, its
compressed Base and RGB 4:4:4 Gain Map JPEGs are assembled in their shared
stored orientation as a standard Ultra HDR intermediate, then converted to
HEIC. The final container reuses the first-assembly HEVC payloads byte-for-byte
after the auxiliary images are authored. The CLI requires `zstd` on `PATH` to
decode OPPO `rear.depth`; JPEG portrait bridging also requires
`ultrahdr_app` on `PATH`.

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

Without `--diagnostics-dir`, Photographic Styles evidence, semantic matte dumps, and
payload encoder inputs use temporary storage and are removed when conversion
exits. With `--diagnostics-dir` in `xdremux-dev`, the complete Styles evidence is retained under
`<diagnostics-dir>/<input-stem>/photographic-styles/runs/<run-identifier>/`, with
`photographic-styles/latest.json` pointing to the newest run.

Validate an output independently:

```bash
swift run xdremux-dev validate-portrait \
  --input IMG_001_apple_portrait.heic \
  --json IMG_001_apple_portrait.validation.json

swift run xdremux-dev portrait-self-test
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
ISO/TS 21496-1 Gain Map and the complete `rear.depth + rear.depth.config +
src.image` bundle. XDRemux uses the RGB 4:4:4 Gain Map JPEG directly from
`src.image`, alongside its matching unblurred Base JPEG; outer ISO metadata
supplies the standardized gain parameters. ImageIO may expose one shared `HDRToneMap:ChannelMetadata`
record when all three raster channels use the same curve; the bridge maps that
record equally across the RGB parameter slots. Batch selection remains
explicit through `--glob '*.jpg'`. JPEG input is accepted only when
`--apple-portrait` is enabled, optionally together with Photographic Styles;
all other product modes keep the existing HEIC-only input contract.

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
- `Sources/XDRemuxExecutable/` owns the thin public entry point.
- `Sources/XDRemuxDevExecutable/` owns the thin developer entry point.
- `Tests/XDRemux*Tests/` contains the SwiftPM regression tests.

Do not place macOS app project files here; app shells belong under `apps/macos/`.

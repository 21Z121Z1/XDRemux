# XDRemux

English Version | [中文版](README.md)

XDRemux converts ProXDR HEIC photos captured on OPPO, OnePlus, and realme devices into standard HDR HEIC files.

It reads the private HDR Gain Map and metadata from the original photo, then repackages them into an HDR HEIC file compliant with ISO 21496-1. The converted photo can be viewed on macOS, iOS, Android, and other systems that support HDR photo display.

## When do I need this tool?

Use XDRemux if you captured ProXDR HEIC photos on an OPPO, OnePlus, or realme phone and want them to keep displaying as HDR photos in other systems or software.

## Usage

The repository root is now a Swift Package. With macOS 15 or later and Swift 6,
run the executable directly after cloning the repository:

```bash
swift run xdremux --help
swift run xdremux convert --input IMG_001.heic --output IMG_001_iso.heic
```

The legacy single-file command remains available as a compatibility entry
point with the same arguments, output, and exit codes:

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

Use `swift build` and `swift test` for development and verification. The
reusable library products are `XDRemuxCore` and `XDRemuxAppleFeatures`; the
public executable is `xdremux`, and internal validation tools live in
`xdremux-dev`.

The Apple Styles and Portrait paths use three isolated helpers built by
SwiftPM. Before using Apple features from a source checkout, run an unscoped
`swift build` once so SwiftPM builds every helper product. Helpers are never
compiled at runtime and helper source is not copied into the App bundle.

### Public CLI

| Scope | Options |
| --- | --- |
| Single file | `convert --input <file> [--output <file>]` |
| Batch | `batch --input-dir <dir> [--output-dir <dir>] [--glob <pattern>] [--jobs <n>]` |
| Writing | `--overwrite`, `--discard-portrait-data` |
| Product modes | `--oppo-compatible`, `--apple-photographic-styles`, `--apple-portrait` |
| Output | `--quiet`, `--verbose`, `--debug`, `--format text|json|jsonl` |
| Language | `--language auto|zh-Hans|en` |

Apple Photographic Styles and Apple Portrait can be combined.
`--oppo-compatible` conflicts with either Apple mode and is rejected during
argument parsing. Quiet mode keeps errors and the final result, verbose mode
adds per-file results, and debug mode adds internal configuration, temporary
paths, and full diagnostic chains.

Text help goes to stdout. Human progress, warnings, and errors go to stderr.
Interactive terminals use in-place progress; redirection, pipes, and CI fall
back to plain lines without ANSI escapes. `json` and `jsonl` go to stdout, and
their field names, event names, and error codes remain English regardless of
the display language:

```json
{"schema_version":1,"event":"conversion_failed","error_code":"source_gain_map_missing","input":"IMG_001.heic","message":"The source photo does not contain a usable HDR Gain Map."}
```

Language priority is `--language`, `XDREMUX_LANGUAGE`, system preferred
languages, then English. Exit codes are `0` success, `1` internal error, `2`
argument error, `3` input error, `4` output or Apple runtime error, `5` partial
batch failure, and `130` for Ctrl+C.

Batch outputs preserve the input tree's relative paths. A rerun skips existing
outputs that pass lightweight validation and regenerates invalid outputs. Each
file is installed atomically from a sibling temporary file, one failure does
not stop the batch, and failures are written to
`<output-dir>/xdremux-failures.json`. Batch recovery no longer depends on a
checkpoint, configuration hash, or mtime journal.

### Developer CLI

Experimental options and validators are available only through `xdremux-dev`:

```bash
swift run xdremux-dev convert \
  --input IMG_001.heic \
  --family x7 \
  --input-processing hybrid \
  --diagnostics-dir diagnostics/

swift run xdremux-dev validate-portrait --input IMG_001_apple_portrait.heic
```

The developer entry also retains `--oppo-compat`, `--oppo-camera-tail`,
`--tmap-format`, `validate-apple`, `validate-portrait`, and
`portrait-self-test`. They do not appear in the public `xdremux --help`.

### Use As A Swift Package

Another SwiftPM project can depend directly on the GitHub repository:

```swift
dependencies: [
    .package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
]
```

Add `.product(name: "XDRemuxCore", package: "XDRemux")` or
`.product(name: "XDRemuxAppleFeatures", package: "XDRemux")` to the consuming
target. The basic conversion entry point is:

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
let request = ConversionRequest(
    input: input,
    output: OutputTarget.file(outputURL).destination(for: input),
    configuration: ConversionConfiguration()
)
let result = try ConversionEngine.convert(request)
```

For Photographic Styles or portrait output, configure the corresponding
`AppleFeatureOptions` and call `AppleFeatureConversionEngine.convert(_:)`.
After the first stable tag, consumers should use a semantic version range
instead of tracking `main` indefinitely.

## Output capabilities

The Swift CLI has three opt-in switches. Apple Photographic Styles and Apple
Portrait are independent, disabled by default, and may be written into one
combined HEIC. `--oppo-compatible` remains mutually exclusive with either
Apple output. With no switches, XDRemux uses the standard ISO default.

| Mode | Switch | Result |
|---|---|---|
| Standard ISO (default) | none | ISO 21496-1 HDR with the source Base Image, channel structure, and non-HDR OPPO/QTI metadata tail; Gain Maps may retain HEVC RExt 4:4:4 when the source supports it |
| OPPO Gallery compatible | `--oppo-compatible` | Main Still Picture 4:2:0 Gain Map for OPPO Gallery, with the OPPO private metadata tail preserved |
| Apple Photographic Styles | `--apple-photographic-styles` | Makes the photo editable with Photographic Styles in Apple Photos, including style switching and tone, color, and intensity controls; everything is generated from the current photo |
| Apple portrait | `--apple-portrait` | Converts an OPPO portrait into a portrait that remains editable in Apple Photos, preserving depth and aperture while improving subject and hair edges |

> [!IMPORTANT]
> Omitting `--output` or `--output-dir` overwrites inputs. Back up originals
> before conversion.

### Default: standard ISO HDR

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic

swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir iso_output/
```

The default does not enable the OPPO-specific compatibility layer. XDRemux
preserves the original Base Image where possible and rebuilds a standard ISO
Gain Map graph. Monochrome sources remain monochrome, while un-downsampled
three-channel sources can retain HEVC Range Extensions 4:4:4. An existing
4:2:0 Gain Map is never advertised as 4:4:4 because discarded chroma cannot be
recovered.

The default preserves non-HDR OPPO/QTI/FileExtendedContainer metadata,
including watermark, master-mode, capture, portrait-editing, and unknown
vendor entries. Private HDR entries under `local.uhdr.*`, `local.hdr.*`,
`src.local.hdr.*`, and `hdr.*` are physically removed, leaving the standard
ISO Gain Map graph as the active HDR display graph.

### `--oppo-compatible`: OPPO Gallery compatibility

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

This mode converts a high-spec Gain Map to Main Still Picture 4:2:0 so OPPO
Gallery can trigger HDR display. It retains the OPPO private metadata tail for
photos intended to return to the OPPO ecosystem.

### `--apple-photographic-styles`: Apple Photographic Styles

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

When enabled, XDRemux builds the data Apple Photos needs to edit Photographic
Styles from the current photo's image, brightness, and color. The result keeps
HDR and lets you switch styles or adjust Tone, Color, and Intensity. No image
content or editing data is borrowed from another photo.

Supporting regions such as people, skin, and sky are added only when they are
actually found in the photo. Missing regions are not replaced with empty masks,
while small but valid regions are kept. If the current macOS version lacks a
required system capability, conversion reports an error instead of creating an
unreliable file.

### `--apple-portrait`: convert OPPO portrait depth

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic

swift run xdremux batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/

# OPPO standard HDR JPEG portraits (writes sibling .heic files and keeps .jpg originals)
swift run xdremux batch \
  --apple-photographic-styles \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/
```

When enabled, XDRemux reads the depth and aperture already stored in the OPPO
portrait and converts them into portrait-editing data understood by Apple
Photos. It analyzes the current photo for the person, skin, hair, and other
local details, and can use aligned OPPO person and hair masks to improve edges.
Unrelated regions are not mixed together, and the original focus and simulated
aperture are preserved where possible.

The production path now parses `rear.depth.config` v1–v4, the complete saved
rank quantizer, near-object/semantic state, and the 22-point aperture curve as
typed data. Apple physical-lens profiles retain only 153 records shown to be
static. `0x0190...0x0199` and `0x01c2...0x01c5` are regenerated for every
photo; XDRemux no longer stores or copies a complete donor REND. A sibling
`*.portrait-manifest.json` records the focus branch, OPPO-domain disparity,
Apple relative disparity, physical profile, dynamic records, and separate
branch/ROI/statistic evidence. PetScene without a saved landmark table now
uses the producer-exact full-image 2% histogram; unresolved face, PetFace, and
near-object refinements remain explicit fallbacks. The firmware-default XHLRB
and Simple Lens Model thresholds are recovered directly, but the active
per-profile `RenderingV...` overrides and clipped-pixel definition are not.
Because iOS `ControlLogicForXHLRB` is not available on macOS, the primary scene
scalar remains explicitly labeled `controlled_corpus_fit`, not Apple-producer
exact.

Use the independent validator to check ImageIO auxiliaries, Focus XMP, REND
round-trip and dynamic relationships, and known donor contamination:

```bash
swift run xdremux-dev validate-portrait \
  --input IMG_001_apple_portrait.heic \
  --json IMG_001_apple_portrait.validation.json
```

The Apple portrait bridge also accepts OPPO HDR JPEGs that contain an ISO/TS
21496-1 Gain Map plus `rear.depth`, `rear.depth.config`, and `src.image`. The
three-channel 4:4:4 Gain Map JPEG is taken directly from `src.image` and paired
with its unblurred Base JPEG in the same stored orientation to form an Ultra HDR
intermediate before HEIC conversion; the outer standard Gain Map supplies the
ISO parameters. When ImageIO represents identical channel curves with one
shared parameter record, XDRemux maps that record equally across all three
channels. This JPEG bridge requires `ultrahdr_app` on `PATH`. Use
`--glob '*.jpg'` to select these JPEG portraits in batch mode. JPEG input is
accepted only when `--apple-portrait` is enabled; Photographic Styles may be
enabled alongside it, while default ISO, Styles-only, and OPPO-compatible modes
retain their existing HEIC-only input contract.

Both Apple switches can be enabled together. XDRemux still creates one HEIC
that keeps HDR, Photographic Styles, and portrait editing. During batch
conversion, an ordinary non-portrait photo is not skipped merely because it has
no depth data; if Photographic Styles is also enabled, a styles-editable photo
is still produced.

Apple output is intended for Apple Photos, while `--oppo-compatible` targets
OPPO Gallery, so they cannot be enabled together. The command fails before
writing a file if both are requested. Portrait blur strength may still vary
between device models and system versions. Passing offline validation does not
constitute a Photos save/reopen or refocus device acceptance result. Payload
transplant now moves the source `hvcC` property associated with each base/gain
item into the portrait scaffold and updates `meta`/`iprp`/`ipco`, `iloc`, and
`mdat`. This accepts 230mm sources with 111/112-byte `hvcC` differences without
attaching preserved HEVC payloads to an incompatible codec graph. Primary EXIF
remains 230mm while the Apple auxiliary saturates at the validated 120mm
physical profile; XDRemux does not invent an Apple 10x lens.

A one-sample macOS Photos run on 2026-07-17 validated the original f/6.3,
f/1.4, f/16, refocus, and save/reopen behavior of the combined 139mm/3x-crop
output, including retained Portrait and Photographic Styles surfaces. This is
not evidence for the full multi-profile matrix or a physical iOS device.

A 230mm/5x-saturated sample also passed macOS Photos on the same date: source
f/10, f/1.4, f/16, background/subject refocus, and saved f/1.4 state after
leaving and reopening the photo. Photos still exposes its low-resolution/
unsupported-format auxiliary badge, so this is not cross-OS or full-matrix
acceptance either.

Those Photos observations predate the current `producer-fallbacks` candidates.
The new files pass offline validation, but UI automation repeatedly failed with
ScreenCaptureKit error `-3811` before import, so no save/reopen result is claimed
for the latest build.

### Python CLI

> [!NOTE]
> Install dependencies first: `pip install pillow-heif Pillow numpy`

```bash
# Single file
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic

# Batch conversion
python3 xdremux/python/XDRemux.py batch --input-dir photo_dump/

# OPPO Gallery-compatible output (--oppo-compat remains as a legacy alias)
python3 xdremux/python/XDRemux.py convert --oppo-compatible --input IMG_001.heic
```

Apple Photographic Styles and Apple Portrait are currently implemented by the
Swift/macOS path. The Python CLI retains its existing HDR conversion support.

### macOS App

Source path:

```text
apps/macos/XDRemuxApp/
```

Build and run locally:

```bash
scripts/build_and_run.sh build
scripts/build_and_run.sh run
scripts/build_and_run.sh verify
```

`build` does not launch, `run` builds and launches, and `verify` also checks the
bundle, helper signatures, and process. `debug` starts LLDB, `logs` filters to
the `com.proxdr.XDRemuxApp` subsystem, `logs --all` shows the process-wide log,
and `clean` removes only XDRemux DerivedData. Default builds are concise and
retain `build.log` plus an `.xcresult`; `--verbose` streams full xcodebuild
output.

The App links `XDRemuxCore` and `XDRemuxAppleFeatures` directly and drives its
ViewModel from the same `ConversionRequest`, structured events, and cancellation
token as the CLI. It never launches or parses the full CLI. Private Vision,
VideoToolbox, and Neutrino work remains isolated in signed executables under
`Contents/Helpers`.

## Swift CLI developer input processing modes

`--input-processing` is an experimental `xdremux-dev` option, not part of the
public user CLI.

```bash
swift run xdremux-dev convert --input IMG_001.heic --input-processing hybrid
```

| Mode | Description |
| --- | --- |
| `hybrid` | Default mode. Preserves the original Base Image and only reprocesses the HDR Gain Map. Non-OPPO outputs keep the original channel layout; OPPO-compatible LHDR uses the verified RGB-copy Gain Map. |
| `system` | Lets the system ImageIO writer produce the final HEIC. This mode re-encodes both the Base Image and the Gain Map, and is useful as a reference for system behavior. |
| `passthrough` | Experimental mode. Rewrites the internal HEIC structure directly for validation and development. Not recommended for normal use. |

## Supported devices

XDRemux is intended for OPPO, OnePlus, and realme devices that can capture ProXDR photos.

The following mainland China models are known to support ProXDR photo capture:

| Brand/series | Models |
| --- | --- |
| OnePlus | OnePlus Ace2 Pro, OnePlus 12, OnePlus Ace3, OnePlus Ace 3V, OnePlus Ace 3 Pro, OnePlus 13, OnePlus Ace 5 series, OnePlus 13T, OnePlus Ace 6, OnePlus Ace 6T, OnePlus Turbo 6, OnePlus 15, OnePlus 15T, OnePlus Ace 5 Supreme Edition |
| OPPO K series | K12, K12x, K13 Turbo series, K15 Pro series |
| OPPO Find series | Find X6, Find X6 Pro, Find N3, Find N3 Flip, Find X7, Find X7 Ultra, Find X8 series, Find N5, Find X8s, Find X9 series, Find N6 |
| OPPO Reno series | Reno10 Pro, Reno10 Pro+, Reno11 Pro, Reno12 series, Reno13 series, Reno14 series, Reno15 series, Reno 16 series |
| realme GT series | realme GT5 series, realme GT5 Pro, realme GT6, realme GT7 Pro, realme GT7 Pro Racing Edition, realme GT7, realme Neo7 Turbo, realme GT8, realme GT8 Pro |
| realme Neo series | realme GT Neo6 SE, realme GT Neo6, realme Neo7, realme Neo7 SE, realme Neo7x, realme Neo8 |
| realme number series | realme 12 Pro, realme 12 Pro+, realme 13 Pro+, realme 13 Pro Supreme Edition, realme 13 Pro, realme 14 Pro+, realme 14 Pro, realme 14, realme 15, realme 15 Pro |

Among them, OPPO Find X8 Ultra, the Find X9 series, and realme GT8 Pro in Ricoh mode support **YCbCr 4:4:4 HDR Gain Map sampling** in their Gain Map implementation.

## Repository structure

| Path | Purpose |
| --- | --- |
| `Package.swift` | Root SwiftPM manifest for two libraries and the `xdremux` / `xdremux-dev` executables. |
| `Sources/XDRemuxCore/` | UI- and CLI-independent conversion models, HDR, HEIF, metadata, and batch core. |
| `Sources/XDRemuxAppleFeatures/` | Apple semantic scene, Photographic Styles, and portrait features. |
| `Sources/XDRemuxCLI/` | Command dispatch, shared argument parsing, localization resources, and terminal/JSON output. |
| `Sources/XDRemuxExecutable/` | Thin entry point for the public `xdremux` executable. |
| `Sources/XDRemuxDevExecutable/` | Internal options and validators for `xdremux-dev`. |
| `xdremux/swift-cli/` | Compatibility forwarding entry point for legacy `swift <file>` commands. |
| `xdremux/python/` | Python CLI and HEIF I/O helper implementation. |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI app shell that consumes the Swift package. |
| `Tests/` | SwiftPM unit tests, Python regressions, and validation harnesses. |
| `fixtures/` | Small test samples and sample notes. |
| `scripts/` | Local build, run, and validation scripts. |
| `experiments/` | Experimental code. |

## Known limitations

- HDR Gain Map and HDR metadata may be lost after editing and saving a converted photo again in OPPO Gallery.

This tool is for technical research only. Back up your original files before conversion. The author assumes no legal responsibility for data loss.

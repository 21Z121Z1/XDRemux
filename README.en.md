# XDRemux

English Version | [中文版](README.md)

XDRemux converts ProXDR photos captured on OPPO, OnePlus, and realme devices into HDR HEIC files with better compatibility.

It reads the private HDR Gain Map and related metadata from a photo and repackages them as an HDR HEIC compliant with ISO/TS 21496-1. The Swift version can also optionally generate Photographic Styles or portrait-editing data for Apple Photos.

## Main features

| Mode | Switch | Purpose |
| --- | --- | --- |
| Standard ISO HDR | default | Converts to ISO/TS 21496-1 HDR HEIC for cross-platform viewing |
| OPPO Gallery compatibility | `--oppo-compatible` | Generates a 4:2:0 Gain Map that is easier for OPPO Gallery to recognize |
| Apple Photographic Styles | `--apple-photographic-styles` | Makes the Photographic Styles editing interface available in Apple Photos |
| Apple Portrait | `--apple-portrait` | Converts OPPO portrait mode data into portrait mode data supported by Apple Photos |

Apple Photographic Styles and Apple Portrait can be enabled together and written into the same HEIC. Apple-related output options cannot be used together with `--oppo-compatible`.

> [!NOTE]
> Apple Photographic Styles and Apple Portrait are currently experimental compatibility features. Results may vary between photos, device models, and system versions.

## Requirements

The Swift version requires:

- macOS 15 or later
- Swift 6
- A ProXDR HEIC photo captured on a supported OPPO, OnePlus, or realme device

Clone the repository and enter the project directory:

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
```

Show the complete command-line help:

```bash
swift run xdremux --help
```

> [!IMPORTANT]
> Omitting `--output` for a single conversion overwrites the input file. Omitting `--output-dir` for a batch conversion writes the results into the input directory. Back up the original photos first.

## Standard ISO HDR

When no feature switch is specified, XDRemux uses the standard ISO HDR mode.

Convert one photo:

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic
```

Convert a directory:

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir iso_output/
```

This mode preserves the original Base Image and non-HDR vendor metadata where possible, and only rebuilds the standard Gain Map structure.

A single-channel source Gain Map remains single-channel. If the source file contains an un-downsampled three-channel 4:4:4 Gain Map, its original channel structure can be preserved.

## OPPO Gallery compatibility mode

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

This mode converts a high-specification Gain Map to HEVC Main Still Picture 4:2:0 and preserves private metadata that may be required by OPPO Gallery where possible.

Use this mode when a photo needs to be imported back into OPPO Gallery. It cannot be used together with Apple Photographic Styles or Apple Portrait.

## Apple Photographic Styles

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux generates the data required by Photographic Styles from the current photo's image, brightness, color, and semantic regions.

The output photo can support the following in Apple Photos:

- Switching Photographic Styles
- Adjusting Tone
- Adjusting Color
- Adjusting style intensity

## Apple Portrait mode

Convert one photo:

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic
```

Convert a directory:

```bash
swift run xdremux batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

Apple Portrait conversion requires the input photo to contain a complete and mutually matching set of OPPO portrait mode information:

- An ISO/TS 21496-1 Gain Map
- `rear.depth`
- `rear.depth.config`
- A complete `src.image`

XDRemux converts the original depth, focus, and simulated-aperture information, and analyzes people, skin, hair, and other regions to improve blur boundaries. The editing capabilities that can be preserved depend on the data actually stored in the source file.

The `src.image` Gain Map in an Apple Portrait input must be readable by macOS ImageIO. RGB 4:4:4 and grayscale Gain Maps are currently supported. A missing, damaged, or nonconforming Gain Map causes the conversion to fail directly.

### JPEG portrait input

Some OPPO portraits use JPEG as the outer container. JPEG input is accepted only when `--apple-portrait` is enabled, and the final output is still converted to HEIC.

JPEG portraits must be selected explicitly in batch mode:

```bash
swift run xdremux batch \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

Without Apple Portrait enabled, standard ISO, OPPO Gallery compatibility, and Photographic Styles-only modes continue to accept HEIC input only.

## Write Photographic Styles and Portrait together

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

Only one HEIC is generated, and XDRemux attempts to preserve all of the following:

- HDR display
- Apple Photographic Styles editing
- Apple Portrait depth editing

During batch conversion, if a normal photo does not contain complete portrait data but Photographic Styles is enabled, XDRemux can still generate a Photographic Styles output for that photo.

## Batch conversion

The default batch behavior includes:

- Using up to four concurrent jobs
- Automatically recording conversion progress
- Continuing after an interruption
- Skipping existing outputs that pass validation
- Retrying failed files the next time the command runs

Common example:

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/ \
  --glob '*.heic' \
  --jobs 4
```

For more checkpoint, overwrite, and diagnostic options, run:

```bash
swift run xdremux --help
```

## Validate output

Validate Apple Photographic Styles or combined output:

```bash
swift run xdremux validate-apple \
  --input IMG_001_apple.heic \
  --expect-portrait \
  --json IMG_001_apple.validation.json
```

Validate only the Apple Portrait structure:

```bash
swift run xdremux validate-portrait \
  --input IMG_001_apple_portrait.heic \
  --json IMG_001_apple_portrait.validation.json
```

The validators inspect HEIC auxiliary images, Focus XMP, and related metadata structures.

Apple Portrait conversion may also generate a sibling `*.portrait-manifest.json` file that records input resources, conversion results, and compatibility diagnostics. This JSON file does not need to be imported into Apple Photos with the photo.

Offline validation only proves that the file structure satisfies the current validation rules. It does not replace import, refocus, save, and reopen testing in Apple Photos.

## Python CLI

The Python CLI provides standard HDR and OPPO Gallery-compatible conversion. It does not include Apple Photographic Styles or Apple Portrait features.

Install the dependencies:

```bash
pip install pillow-heif Pillow numpy
```

Convert one photo:

```bash
python3 xdremux/python/XDRemux.py convert \
  --input IMG_001.heic
```

Convert a directory:

```bash
python3 xdremux/python/XDRemux.py batch \
  --input-dir photo_dump/
```

Create OPPO Gallery-compatible output:

```bash
python3 xdremux/python/XDRemux.py convert \
  --oppo-compatible \
  --input IMG_001.heic
```

## macOS App

The macOS App source is located at:

```text
apps/macos/XDRemuxApp/
```

Build and run it locally:

```bash
scripts/build_and_run.sh run
```

## Use as a Swift Package

Other SwiftPM projects can depend directly on this repository:

```swift
dependencies: [
    .package(
        url: "https://github.com/21Z121Z1/XDRemux.git",
        branch: "main"
    )
]
```

Use `XDRemuxCore` for basic HDR conversion:

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

For Apple Photographic Styles or Apple Portrait, add the `XDRemuxAppleFeatures` product and use `AppleFeatureConversionEngine`.

## Development

Build:

```bash
swift build
```

Run the tests:

```bash
swift test
```

Main directories:

| Path | Purpose |
| --- | --- |
| `Sources/XDRemuxCore/` | HDR, HEIF, metadata, and batch-conversion core |
| `Sources/XDRemuxAppleFeatures/` | Apple Photographic Styles and Portrait features |
| `Sources/XDRemuxCLI/` | Swift command-line entry point |
| `xdremux/python/` | Python CLI |
| `apps/macos/XDRemuxApp/` | macOS App |
| `Tests/` | Automated tests |
| `scripts/` | Build and validation scripts |

## Device and file compatibility

XDRemux targets OPPO, OnePlus, and realme devices capable of capturing ProXDR photos.

The project does not rely on a fixed device whitelist. Instead, it checks the Gain Map and vendor metadata actually present in each input file. Different firmware versions on the same device may produce different structures. Files that do not meet the current input requirements fail with an explicit error.

Not every system gallery or third-party application supports ISO/TS 21496-1, 4:4:4 Gain Maps, Apple Photographic Styles, or Apple Portrait data.

## Known limitations

- Editing and saving a converted photo in OPPO Gallery may remove the standard HDR Gain Map or HDR metadata.
- Conversion may overwrite the input file. Keep an unmodified copy of the original photo.
- Applications may interpret HDR peak brightness, color management, and Gain Maps differently.

This project is intended for technical research. Converted results should not be used as the only copy of an original photo.

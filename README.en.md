# XDRemux

English | [简体中文](README.md)

XDRemux converts vendor HDR photos to standardized HDR HEIC and converts supported Android Motion Photos to Apple Live Photo.

For ProXDR input, XDRemux reads the source Gain Map and related metadata and writes an HDR HEIC for the ISO/TS 21496-1 representation.

For Motion Photo input, XDRemux creates an Apple Live Photo HEIC + MOV pair. The normal Live Photo path preserves the HDR still, the cover-frame presentation timestamp (PTS), and compressed video and audio samples.

## Main features

| Feature | Selection | Output |
| --- | --- | --- |
| Standard HDR | default | ISO/TS 21496-1 HDR HEIC |
| Motion Photo to Live Photo | automatic | HEIC + MOV |
| OPPO Gallery compatibility | `--oppo-compatible` | compatibility-oriented HDR HEIC |
| Apple Photographic Styles | `--apple-photographic-styles` | HEIC with Apple style-editing resources |
| Apple Portrait | `--apple-portrait` | HEIC with Apple portrait-editing resources |
| Classification | `categorize` or `batch --categorize` | asset-type and capture-mode directories |

Apple-specific editing features have separate support and validation boundaries. See the [Apple features guide](docs/apple-features.en.md).

## Requirements

The Swift package requires macOS 15 or later and uses Swift tools 6.0.

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
swift run xdremux --help
```

For CPU-heavy Photographic Styles work:

```bash
swift build -c release
```

The Python package requires Python 3.11 or later.

```bash
pip install -e .
xdremux-py --help
```

## Standard HDR conversion

Convert one photo:

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

Convert a directory:

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

The standard path preserves source compressed image data when the selected conversion path permits it. The converter writes standardized Gain Map metadata and preserves supported source Gain Map characteristics.

> [!IMPORTANT]
> For normal HDR input, omitting `--output` can target the input file. Keep an unmodified source when the original file is important.

## Motion Photo to Apple Live Photo

Motion Photo detection is automatic for supported JPEG and HEIC/HEIF inputs.

```bash
swift run xdremux convert --input IMG_001.jpg
```

The output pair uses one basename:

```text
IMG_001.heic
IMG_001.mov
```

The normal Live Photo path preserves:

- the HDR still and Gain Map when present;
- the resolved source cover-frame PTS;
- Apple `still-image-time`;
- compressed video samples;
- compressed audio samples when present;
- supported orientation and geometry metadata.

The source Motion Photo is not modified.

If an implicit output name collides with an existing HEIC/HEIF or companion MOV, XDRemux selects the next available basename such as `IMG_001 (2)`.

If you explicitly set `--output`, XDRemux refuses to overwrite an existing Live Photo output pair.

Batch conversion can contain normal HDR photos and Motion Photos:

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

See the [CLI reference](docs/cli.en.md) for batch discovery, checkpoint, and provenance rules.

## OPPO Gallery compatibility

Use `--oppo-compatible` when the output must use the OPPO compatibility path:

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

This path can reduce Gain Map chroma representation for compatibility. A reduction cannot reconstruct discarded source chroma later.

## Apple Photographic Styles

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

The current default style-data producer is `constrained-solver` when Photographic Styles is enabled.

The repository also contains research and diagnostic producer paths. They are not the normal default.

See [Apple features](docs/apple-features.en.md) for supported combinations and acceptance limits.

## Apple Portrait

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

The source photo must contain the portrait resources required by the converter.

See [Apple features](docs/apple-features.en.md) for the current resource and validation boundary.

## Classification

Classify without converting:

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

Preview the Swift plan:

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/ \
  --dry-run
```

A validated Live Photo HEIC and MOV remain one asset during classification.

Batch conversion can classify its outputs with `--categorize`:

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/ \
  --categorize
```

Python provides the same classification command family:

```bash
python3 -m xdremux_py categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

## Python CLI

Python supports cross-platform HDR conversion, Motion Photo to Live Photo conversion, and classification.

```bash
xdremux-py convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic

xdremux-py convert --input IMG_001.jpg
```

Python conversion does not require Apple platform frameworks. macOS Apple frameworks are used by separate compatibility tests where applicable.

Python does not generate Photographic Styles or Apple Portrait data.

See the [CLI reference](docs/cli.en.md) for differences between Swift and Python batch behavior.

## macOS app

Build and run the app:

```bash
scripts/build_and_run.sh run
```

The app links the Swift package directly.

## Swift package

Public package products are:

- `XDRemuxCore`
- `XDRemuxAppleFeatures`
- `xdremux`

Example:

```swift
.package(
    url: "https://github.com/21Z121Z1/XDRemux.git",
    branch: "main"
)
```

Use `XDRemuxCore` for the standard conversion API and `XDRemuxAppleFeatures` for Apple-specific conversion engines.

## Validation

The repository uses unit tests, repository policy tests, real Motion Photo fixtures, macOS framework validation, and device validation when a claim requires it.

The public Motion Photo corpus is versioned under `fixtures/`. The strict gates verify exact fixture identity and applicable Live Photo contracts such as timing, asset identity, Gain Map preservation, compressed-sample passthrough, and publication safety.

See the [testing policy](docs/quality/testing.en.md).

## Documentation

| Document | Purpose |
| --- | --- |
| [Documentation index](docs/README.en.md) | All current and historical technical documents |
| [CLI reference](docs/cli.en.md) | Commands, defaults, and output rules |
| [Apple features](docs/apple-features.en.md) | Photographic Styles and Apple Portrait |
| [Supported devices](docs/supported-devices.en.md) | ProXDR compatibility boundary |
| [Development](docs/development.en.md) | Package structure and build workflow |
| [Testing policy](docs/quality/testing.en.md) | Required verification evidence |
| [Technical implementation](docs/xdremux/README.en.md) | Current implementation contracts |
| [Technical writing guide](docs/style-guide.en.md) | Documentation terminology and STE principles |

## Known limitations

- HDR rendering depends on the operating system and viewer.
- A vendor gallery can remove standardized HDR metadata when it edits and saves a converted file.
- Apple-specific editing behavior depends on the Apple Photos and framework version.
- A supported device model does not guarantee that every file contains the data required by every conversion feature.
- Structural validation does not replace device evidence for a device-dependent claim.

Keep the original file when source data is important.

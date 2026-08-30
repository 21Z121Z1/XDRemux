# XDRemux

English | [简体中文](README.md)

XDRemux converts vendor-specific HDR photos and Android Motion Photos into formats that are compatible with standard HDR viewers and Apple Photos.

For ProXDR photos, XDRemux reads the original HDR Gain Map and related metadata and remuxes them into an HDR HEIC that conforms to ISO/TS 21496-1.

For Motion Photos, XDRemux creates an Apple Live Photo resource pair while preserving the HDR still image, the cover-frame presentation timestamp, and the original compressed video and audio samples.

## Features

| Feature | Usage | Output |
| --- | --- | --- |
| Standard HDR | Default | ISO/TS 21496-1 HDR HEIC |
| Motion Photo conversion | Automatic | Apple Live Photo HEIC + MOV |
| OPPO Gallery compatibility | `--oppo-compatible` | HDR HEIC with a 4:2:0 Gain Map |
| Apple Photographic Styles | `--apple-photographic-styles` | HEIC with Photographic Styles editing data |
| Apple Portrait | `--apple-portrait` | HEIC with Apple Photos portrait editing data |
| Photo classification | `categorize` or `batch --categorize` | Asset and capture-mode folders |

Photographic Styles and Apple Portrait use Apple-specific editing metadata. They are separate from the standard HDR and Live Photo conversion paths.

## Requirements

The Swift implementation requires:

- macOS 15 or later
- Swift 6

Clone and build the project:

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
```

For normal use, a release build is recommended:

```bash
swift build -c release
```

Show the command hierarchy:

```bash
swift run xdremux --help
```

## HDR conversion

Convert one ProXDR photo:

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

The standard path preserves the original encoded image data and reconstructs the HDR Gain Map metadata in the ISO/TS 21496-1 representation.

Source Gain Map characteristics are preserved when the output format supports them. This includes single-channel Gain Maps and high-precision three-channel Gain Maps.

> [!IMPORTANT]
> A normal HEIC conversion without `--output` can replace the input file. Keep an unmodified copy of important source photos.

## Motion Photo to Apple Live Photo

XDRemux automatically detects supported Motion Photos. No additional option is required.

```bash
swift run xdremux convert \
  --input IMG_001.jpg
```

The output is an Apple Live Photo resource pair:

```text
IMG_001.heic
IMG_001.mov
```

The two files share the Apple Live Photo asset identifier and must remain together.

The conversion preserves the following data:

- HDR still-image data, including the Gain Map
- The source cover-frame presentation timestamp (PTS)
- The corresponding Apple `still-image-time`
- Compressed video samples
- Compressed audio samples
- Supported orientation and geometry metadata

The video and audio paths use compressed-sample passthrough. XDRemux does not re-encode these samples during the normal Live Photo remux path.

The source Motion Photo is not modified.

If the destination already contains either member of the requested Live Photo pair, XDRemux selects the next available basename:

```text
IMG_001 (2).heic
IMG_001 (2).mov
```

An explicit `--output` never silently replaces an existing HEIC or its companion MOV.

Batch conversion uses the same automatic detection:

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

A batch can contain normal HDR photos and Motion Photos. XDRemux selects the applicable conversion path for each input.

## OPPO Gallery compatibility

Some OPPO Gallery versions do not accept high-specification Gain Maps.

Use the compatibility mode when the converted file must remain editable or viewable in OPPO Gallery:

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

This mode writes the Gain Map as HEVC Main Still Picture 4:2:0 and preserves the required private metadata when available.

This conversion reduces the Gain Map chroma representation. It is not reversible to the original higher-precision representation.

## Apple Photographic Styles

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux can generate the metadata and image resources used by the Photographic Styles editing interface in Apple Photos.

This is an Apple-specific compatibility feature. Its implementation and validation status are documented separately in [Apple features](docs/apple-features.md).

## Apple Portrait

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

When the source photo contains the required portrait resources, XDRemux can convert depth, focus, aperture, and semantic data into a representation that Apple Photos can use for portrait editing.

The available editing functions depend on the data present in the source photo.

See [Apple features](docs/apple-features.md) for the current input requirements and validation boundary.

## Classification

XDRemux can classify photos without changing their encoded image data:

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

Use `--dry-run` to inspect the planned file layout:

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/ \
  --dry-run
```

The physical layout separates static photos and Live Photos, then groups assets by their primary capture mode.

A validated Live Photo HEIC and MOV remain one asset and move together.

Additional properties such as HDR, Gain Map, portrait data, and vendor metadata remain classification tags instead of additional directory levels.

## Python CLI

The Python implementation supports cross-platform HDR conversion, Motion Photo to Live Photo conversion, and classification.

It does not require Apple frameworks for conversion.

Install the package:

```bash
pip install -e .
```

Convert an HDR photo:

```bash
xdremux-py convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

Convert a Motion Photo:

```bash
xdremux-py convert \
  --input IMG_001.jpg
```

The Python Live Photo path preserves the HDR still-image data, cover-frame timing, and compressed media samples.

Apple platform frameworks are used only by the macOS compatibility tests. They are not runtime dependencies of the Python converter.

The package can also run directly from the repository:

```bash
python3 -m xdremux_py --help
```

## macOS App

Build and run the macOS application:

```bash
scripts/build_and_run.sh run
```

The application provides the conversion and classification workflows through a graphical interface.

## Swift Package

XDRemuxCore and XDRemuxAppleFeatures are available as Swift Package products.

```swift
.package(
    url: "https://github.com/21Z121Z1/XDRemux.git",
    branch: "main"
)
```

Use `XDRemuxCore` for the standard conversion pipeline:

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

Apple-specific features are provided by `XDRemuxAppleFeatures`.

## Validation

XDRemux uses automated structural tests and real-file integration tests.

The Motion Photo test corpus contains multiple JPEG and HEIC/HEIF layouts. The CI pipeline verifies:

- Source-file integrity
- Motion Photo resource boundaries
- Cover-frame timing
- Live Photo asset identifiers
- Apple `still-image-time` metadata
- HDR Gain Map preservation
- Compressed video sample equality
- Compressed audio sample equality
- Live Photo structural validity
- PhotoKit compatibility on macOS

The downloadable device-validation bundles are generated by the same production conversion engine.

## Documentation

| Document | Description |
| --- | --- |
| [CLI reference](docs/cli.md) | Commands, options, defaults, and exit behavior |
| [Apple features](docs/apple-features.md) | Photographic Styles and Apple Portrait |
| [Supported devices](docs/supported-devices.md) | ProXDR capture compatibility |
| [Development](docs/development.md) | Package structure and development workflow |
| [Technical implementation](docs/xdremux/README.md) | HDR, HEIF, Gain Map, and container implementation |

## Known limitations

- Application support for ISO/TS 21496-1 HDR varies.
- HDR rendering can differ between operating systems and image viewers.
- Editing and saving a converted HDR photo in a vendor gallery can remove standardized HDR metadata.
- Apple-specific editing metadata depends on the behavior of the Apple Photos version that reads it.
- A supported device does not guarantee that every photo from that device has the required source metadata.

Keep the original photo when the source data is important.

## License

See [LICENSE](LICENSE).

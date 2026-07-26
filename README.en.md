# XDRemux

English | [简体中文](README.md)

Convert ProXDR photos from OPPO, OnePlus, and realme phones into the standard HDR photo format (ISO/TS 21496-1), so they display properly on iPhone, on Mac, and anywhere else that supports it.

ProXDR keeps its HDR information in the manufacturer's private data, so anywhere outside the stock gallery you get an ordinary photo with the highlights flattened. XDRemux translates it into the standard format without re-encoding a single byte of the picture.

## Quick start

Requires macOS 15 or newer and Swift 6.

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
```

Convert one photo:

```bash
.build/debug/xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

Convert a whole directory:

```bash
.build/debug/xdremux batch --input-dir photo_dump/ --output-dir converted/
```

> [!WARNING]
> `convert` without `--output`, and `batch` without `--output-dir`, **overwrite the original files**. Back up your originals first.

## What you can produce

| What you want | What to pass |
| --- | --- |
| Standard ISO HDR | nothing — this is the default |
| HDR that OPPO Gallery also recognizes | `--oppo-compatible` |
| Apple Photos Photographic Styles editing | `--apple-photographic-styles` |
| Apple Photos portrait depth editing | `--apple-portrait` |
| Files sorted by shooting mode | `categorize` or `batch --categorize` |

By default only the HDR data is rewritten (the industry calls it a gain map: it records how much brighter each pixel should get). The picture, the watermark, the master-mode settings all stay exactly as they were. HDR precision is preserved too; `--oppo-compatible` has to drop it a notch before OPPO Gallery will read it, and there is no way back.

Every option, default, and exit code is in the [CLI reference](docs/cli.en.md), or run `swift run xdremux --help`.

## Sorting by shooting mode

Reads the shooting mode recorded in each photo and copies it into the matching Chinese-named folder (`人像` portrait, `夜景` night, `大师模式` master mode, and so on). It only copies — sources are never modified or deleted:

```bash
swift run xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run
```

`--dry-run` prints the plan without touching anything. Passing `--categorize` to `batch` files the converted results into those same folders instead. The Python version behaves identically:

```bash
python3 xdremux/python/XDRemux.py categorize --input photo_dump/ --output-dir categorized/
```

Photos whose mode cannot be read stay in the output root and are not counted as failures.

## Apple Photographic Styles and portrait

Make a converted photo support Photographic Styles and portrait depth editing in Apple Photos. All of that data is computed from your own photo — no separate iPhone photo to copy from. Portrait needs a source shot in portrait mode with its depth data still intact; photos edited afterwards may have lost it. Both features can be enabled together, and neither can be combined with `--oppo-compatible`.

> [!IMPORTANT]
> Neither feature has passed production acceptance. The checking reaches the file-structure level only: the file opens, the data is there. What has **not** been verified is whether it survives being imported into Photos on a real device, edited, saved, and reopened. See the [Apple features guide](docs/apple-features.en.md).

Generating Photographic Styles data is CPU-heavy. Use a release build for batches; it is several times faster than the default debug build:

```bash
swift build -c release
.build/release/xdremux batch --apple-photographic-styles --input-dir photo_dump/ --output-dir styled/
```

## macOS app

```bash
scripts/build_and_run.sh run
```

The app covers both conversion and shooting-mode sorting, with drag-and-drop, previews, a concurrency setting, resumable batches, and Reveal in Finder.

## Python CLI

Cross-platform, HDR conversion only — no Apple features. Requires Python 3.11 or newer.

```bash
pip install -r xdremux/python/requirements.txt
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic --output IMG_001_hdr.heic
```

## Using it as a Swift package

```swift
.package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
```

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
let result = try ConversionEngine.convert(
    ConversionRequest(
        input: input,
        output: OutputTarget.file(outputURL).destination(for: input),
        configuration: ConversionConfiguration()
    )
)
```

The Apple features live in the `XDRemuxAppleFeatures` product, behind `AppleFeatureConversionEngine`. See the [development guide](docs/development.en.md).

## Documentation

| Document | Covers |
| --- | --- |
| [CLI reference](docs/cli.en.md) | Every command, option, default, and exit code |
| [Apple features](docs/apple-features.en.md) | What Styles and portrait can do, and how far they have been verified |
| [Supported devices](docs/supported-devices.en.md) | Phones known to shoot ProXDR |
| [Development guide](docs/development.en.md) | Module layout, Swift package integration, build workflows |
| [Technical implementation](docs/xdremux/README.en.md) | HDR, HEIF file structure, and how the ISO standard is implemented |

## Known limitations

- Re-editing a converted photo in OPPO Gallery and saving it can drop the standard HDR gain map.
- Applications disagree about HDR peak brightness, colour management, and gain map interpretation, so the same photo can look different in different places.
- A phone being on the supported list does not mean every photo it takes carries a convertible gain map. The real answer depends on the shooting mode, firmware version, and edit history.
- This is a technical research project. Do not let a converted file be the only copy of a photo.

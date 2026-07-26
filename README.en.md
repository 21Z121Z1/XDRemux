# XDRemux

English | [简体中文](README.md)

Convert ProXDR photos from OPPO, OnePlus, and realme phones into standard HDR HEIC that any ISO/TS 21496-1 viewer can display.

On these phones the HDR information lives in a vendor-private block, so a ProXDR photo looks like an ordinary SDR image anywhere outside the stock gallery. XDRemux reads the private gain map and metadata and repackages them as a standard ISO/TS 21496-1 structure. The primary image payload is preserved byte for byte; only the gain map and the container structures it depends on are rebuilt.

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

The default mode preserves the original primary image and the non-HDR vendor metadata, rebuilding only the gain map structure. A source gain map that is full-resolution 4:4:4 stays 4:4:4; `--oppo-compatible` downsamples it to the 4:2:0 form OPPO Gallery needs, which cannot be undone.

Every option, default, and exit code is in the [CLI reference](docs/cli.en.md), or run `swift run xdremux --help`.

## Sorting by shooting mode

Reads the shooting mode from the EXIF UserComment and copies photos into Chinese-named folders (`人像`, `夜景`, `大师模式`, and so on). It only copies — sources are never modified or deleted:

```bash
swift run xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run
```

`--dry-run` prints the plan without touching anything. Passing `--categorize` to `batch` files the converted results into those same folders instead. The Python version behaves identically:

```bash
python3 xdremux/python/XDRemux.py categorize --input photo_dump/ --output-dir categorized/
```

Photos whose mode cannot be read stay in the output root and are not counted as failures.

## Apple Photographic Styles and portrait

XDRemux generates the Photographic Styles and portrait editing data Apple Photos expects from the photo itself, with no Apple donor photo involved. The portrait feature needs a source photo carrying the complete OPPO depth bundle (`rear.depth`, `rear.depth.config`, `src.image`). Both features can be enabled together and written into one HEIC; both are mutually exclusive with `--oppo-compatible`.

> [!IMPORTANT]
> Neither feature has passed production acceptance. The reproducible evidence today covers offline container structure, ImageIO, and the repository validators — it does **not** include importing into Photos on a real device, editing, saving, quitting, and reopening. See the [Apple features guide](docs/apple-features.en.md) for exactly what has and has not been proven.

Solving for Photographic Styles is compute-heavy. Use a release build for batches; it is several times faster than the default debug build:

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
| [Apple features](docs/apple-features.en.md) | What Styles and portrait can do, and their verification status |
| [Supported devices](docs/supported-devices.en.md) | Phones known to shoot ProXDR |
| [Development guide](docs/development.en.md) | Module layout, Swift package integration, build workflows |
| [Technical implementation](docs/xdremux/README.en.md) | HDR, HEIF, and ISO container behaviour |

## Known limitations

- Re-editing a converted photo in OPPO Gallery and saving it can drop the standard HDR gain map.
- Applications disagree about HDR peak brightness, colour management, and gain map interpretation, so the same photo can look different in different places.
- A phone being on the supported list does not mean every photo it takes carries a convertible gain map. The real answer depends on the shooting mode, firmware version, and edit history.
- This is a technical research project. Do not let a converted file be the only copy of a photo.

# XDRemux

English | [简体中文](README.md)

XDRemux converts ProXDR photos captured by OPPO, OnePlus, and realme devices into HDR HEIC files that are easier for other systems to recognize.

It can produce standard ISO HDR output, preserve OPPO Gallery compatibility, or add Photographic Styles and portrait editing data for Apple Photos.

## Features

| Mode | Purpose |
| --- | --- |
| Standard HDR | Display HDR on systems that support ISO HDR Gain Maps |
| OPPO compatible | Preserve HDR display compatibility in OPPO Gallery |
| Apple Photographic Styles | Use Photographic Styles editing in Apple Photos |
| Apple Portrait | Continue adjusting depth and focus in Apple Photos |

Standard HDR is the default. Apple Photographic Styles and Apple Portrait can be combined; OPPO-compatible output cannot be combined with either Apple mode.

## Requirements

- macOS 15 or later.
- A Swift 6 toolchain.
- Apple Portrait requires `zstd`; JPEG portrait bridging also requires `ultrahdr_app` on `PATH`.

Before using an Apple feature from a source checkout, run one complete `swift build`.

## Quick start

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
```

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
  --output-dir converted/
```

> [!IMPORTANT]
> Omitting `--output` or `--output-dir` overwrites the input. Back up original photos before conversion.

## Common modes

Standard HDR:

```bash
swift run xdremux convert --input IMG_001.heic --output IMG_001_iso.heic
```

OPPO Gallery compatibility:

```bash
swift run xdremux convert --oppo-compatible --input IMG_001.heic --output IMG_001_oppo.heic
```

Apple Photographic Styles:

```bash
swift run xdremux convert --apple-photographic-styles --input IMG_001.heic --output IMG_001_styles.heic
```

Apple Portrait:

```bash
swift run xdremux convert --apple-portrait --input IMG_001.heic --output IMG_001_portrait.heic
```

Show all public options:

```bash
swift run xdremux --help
```

## Supported inputs

XDRemux targets OPPO, OnePlus, and realme devices that can capture ProXDR HEIC. Different models may use different Gain Map encodings and vendor metadata. See [Supported devices](docs/supported-devices.en.md) for the known device list.

Apple Portrait requires recoverable depth data in the source photo. Enabling the option does not turn an ordinary photo into a portrait photo.

## Known limitations

- Apple Photographic Styles and Apple Portrait remain experimental and may vary across devices and macOS or iOS releases.
- Editing and saving a converted photo in OPPO Gallery may remove its HDR Gain Map or HDR metadata.
- Offline container validation does not replace real Apple Photos or OPPO Gallery display and save/reopen testing.
- The project is currently distributed as source and does not provide a signed universal installer.

## Documentation

- [Complete CLI reference](docs/cli.en.md)
- [Apple Photographic Styles and Portrait](docs/apple-features.en.md)
- [Development, builds, and Swift Package integration](docs/development.en.md)
- [Supported devices](docs/supported-devices.en.md)
- [Technical implementation and validation notes](docs/xdremux/README.en.md)
- [中文文档](docs/README.md)

## Disclaimer

This tool is provided for technical research. Back up original files before conversion. The author assumes no responsibility for data loss caused by its use.

# XDRemux

English | [简体中文](README.md)

XDRemux converts supported vendor ProXDR photos to ISO/TS 21496-1 HDR HEIC. It also converts supported Android Motion Photos to Apple Live Photo pairs.

The Rust `xdremux` CLI detects source type, HDR structure, and conversion routing. The macOS app uses the same product implementation. Source compatibility depends on the actual file structure and available resources, not only the device name.

## Supported tasks

| Task | Result or command |
| --- | --- |
| Convert a ProXDR still | HDR HEIC with an ISO Gain Map. |
| Convert a supported Motion Photo | A matched HEIC and MOV Live Photo pair. |
| Package a Live Photo | `--live-photo-output pvt` creates a directory with the validated pair and metadata. |
| Request OPPO Gallery compatibility | `--oppo-compatible` applies to ProXDR stills. |
| Add Apple editing resources | Portrait, Photographic Styles, and Texture Styles require the documented source and macOS capabilities. |
| Add pending Cinematic metadata | Experimental `--apple-cinematic` prepares ordinary video for a separate native processing step. |
| Convert a collection | `batch` plans outputs and reports each result. |
| Sort assets | `categorize` or `batch --categorize` uses asset type and capture mode. |
| Examine files | `inspect` reports source facts. `validate` checks supported outputs. |

The complete Mac product targets Apple silicon (`arm64`) and macOS 27. The Rust core also has Linux, Windows, and macOS builds and portable tests. A portable build does not establish Apple framework support or Photos editing behavior.

See [supported devices and source evidence](docs/supported-devices.en.md) and [Apple feature requirements](docs/apple-features.en.md) before selecting an editing feature.

## Build the Rust CLI

Install a current Rust toolchain and libheif with HEVC support for portable conversion. The complete Mac package also requires the Apple adapter. Its build uses Xcode 27 and a macOS 27 deployment target.

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
cargo build --locked --release -p xdremux-cli
./target/release/xdremux --help
```

The following examples assume that `xdremux` is on `PATH`. For a source build, substitute `./target/release/xdremux` for `xdremux`.

For development, invoke the CLI through Cargo:

```bash
cargo run --locked -p xdremux-cli -- --help
```

For the app, adapter, and relocatable package, use the [development guide](docs/development.en.md).

## Convert one photo

> [!CAUTION]
> For a ProXDR still, omitting `--output` replaces the input after conversion succeeds. Specify a separate output to retain the original file.

Convert a supported ProXDR still:

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

Convert a supported Motion Photo:

```bash
xdremux convert --input IMG_001.jpg
```

Motion Photo conversion creates a matched HEIC and MOV pair and keeps the source unchanged. Without `--output`, the command chooses names that do not collide with existing pair members.

For a ProXDR still intended for OPPO Gallery, add the compatibility option:

```bash
xdremux convert --input IMG_001.heic --output IMG_001_oppo.heic --oppo-compatible
```

Motion Photo conversion rejects `--oppo-compatible`. PVT output and Apple editing options have separate requirements in the [CLI reference](docs/cli.en.md).

## Convert a batch

```bash
xdremux batch --input-dir photo_dump/ --recursive --output-dir converted/
```

Batch plans output paths before it writes files. It limits concurrency with `--jobs` and reports failures per item. Use `--json` for a structured receipt.

Live Photo checkpoint reuse requires matching source and output identity. It does not mean that every existing output can be skipped. See [batch options and recovery](docs/cli.en.md#batch).

## Categorize assets

```bash
xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run
xdremux categorize --input photo_dump/ --output-dir categorized/
```

The first command reports the plan without publishing files. The second copies assets into classification folders. A validated Live Photo still and MOV remain one asset. Use `batch --categorize` to place converted results in those folders.

## Inspect and validate

```bash
xdremux inspect IMG_001.heic --json
xdremux validate IMG_001_hdr.heic --json
```

`inspect` reports parsed source facts. Plain `validate` checks supported HDR HEIF, a Live Photo pair, or a PVT package. Structural validation does not establish gallery display or interactive editing.

## Apple editing features

Rust selects the feature, prepares resources, validates results, and publishes output. The Swift adapter performs operations that require Apple frameworks.

`--apple-portrait` requests the complete portrait resource set. `--apple-portrait-oppo` uses OPPO depth and available masks without Vision segmentation. The latter has reduced effects and still requires public ImageIO and Core Image APIs on macOS.

`--apple-styles` generates Photographic Styles and inferred Texture Styles on the supported macOS host. `--apple-cinematic` generates pending metadata for ordinary video. A pending Cinematic state does not guarantee that Photos can process every input.

Use the [Apple feature guide](docs/apple-features.en.md) and [Cinematic contract](docs/cinematic.en.md) for source requirements and validation limits.

## Code and documentation

The CLI calls the Rust runtime. The runtime executes plans from the engine through media crates and platform adapters. Product policy stays in Rust. Python tools support research and model training.

| Document | Use it for |
| --- | --- |
| [CLI reference](docs/cli.en.md) | Options, defaults, output safety, and exit codes. |
| [Architecture](docs/architecture.en.md) | Module responsibilities and media contracts. |
| [Development](docs/development.en.md) | Builds, packaging, and contributor checks. |
| [Testing policy](docs/quality/testing.en.md) | Evidence required for each type of change. |
| [Documentation index](docs/README.en.md) | Research tools, validation records, and other guides. |

The versioned media in `fixtures/` supports real-file tests. Fixture identity and provenance remain part of the test contract.

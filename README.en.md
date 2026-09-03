# XDRemux

English | [简体中文](README.md)

XDRemux converts supported vendor HDR photos to ISO/TS 21496-1 HDR HEIC and supported Android Motion Photos to Apple Live Photo.

There is one product entry point: the Rust `xdremux` CLI. Input type, source generation, HDR/Gain Map structure, and Motion Photo routing are detected automatically.

## What it does

| Input / intent | Result |
| --- | --- |
| ProXDR photo | ISO/TS 21496-1 HDR HEIC |
| Supported Motion Photo | Apple Live Photo HEIC + MOV |
| `--oppo-compatible` | ProXDR output intended for OPPO Gallery |
| `categorize` / `batch --categorize` | Asset-type and capture-mode folders |
| `inspect` / `validate` | Source inspection and independent output validation |

The normal path has no device-generation, codec, Gain Map layout, camera-tail, or routing switches. Those are implementation decisions derived from the source and requested result.

## Build

A current Rust toolchain and a libheif installation with HEVC support are required for the portable conversion stack.

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
cargo build --release -p xdremux-cli
./target/release/xdremux --help
```

For development you can run the binary directly through Cargo:

```bash
cargo run -p xdremux-cli -- --help
```

## Convert

Standard ProXDR conversion:

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

Supported Motion Photos use the same command and are detected automatically:

```bash
xdremux convert --input IMG_001.jpg
```

A Motion Photo produces a matching HEIC + MOV Live Photo pair. The source Motion Photo is not modified.

For OPPO Gallery oriented ProXDR output:

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic \
  --oppo-compatible
```

`--oppo-compatible` applies to ProXDR still images, not Motion Photo conversion.

> [!IMPORTANT]
> For a ProXDR still image, omitting `--output` targets the input path and publishes the replacement atomically. Keep an original copy when the source file matters.

## Batch

```bash
xdremux batch \
  --input-dir photo_dump/ \
  --recursive \
  --output-dir converted/
```

Batch supports repeated files/directories, bounded `--jobs`, deterministic output planning, per-item failure isolation, checkpoint/resume, provenance-checked reuse, structured JSON receipts, and optional `--categorize` publication.

See [`docs/cli.en.md`](docs/cli.en.md) for the complete command contract.

## Categorize

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

A validated Live Photo HEIC and MOV are treated as one asset. Use `--dry-run` to inspect the plan without publishing files.

## Inspect and validate

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json

xdremux validate output.heic
xdremux validate output.heic --json
```

`inspect` reports parsed source facts. `validate` automatically checks ISO HDR HEIF or a Live Photo pair.

## Apple editing features

Photographic Styles and Apple Portrait currently remain behind a migration boundary. The target architecture keeps product policy, orchestration, data models, and the CLI in Rust while a narrow Apple-native adapter invokes platform frameworks such as Core Image, Vision, Core ML, and AVFoundation.

See [`docs/apple-features.en.md`](docs/apple-features.en.md) for the current support and acceptance boundary.

## Architecture

```text
xdremux-cli
    ↓
xdremux-runtime
    ↓
xdremux-engine
    ↓
source / classification
motion-photo / hdr / metadata
container / heif / codec / format
    ↓
portable providers + platform adapters
```

New product behavior belongs in this Rust stack. Migration or research implementations do not define the public CLI contract.

## Documentation

- [CLI reference](docs/cli.en.md) — commands, defaults, exit status, and batch reliability
- [Apple features](docs/apple-features.en.md) — Photographic Styles and Portrait migration boundary
- [Supported devices](docs/supported-devices.en.md) — source compatibility evidence
- [Development](docs/development.en.md) — architecture, ownership, and build/test workflow
- [Testing policy](docs/quality/testing.en.md) — required validation evidence
- [Documentation index](docs/README.en.md) — additional technical documentation

The versioned media corpus under `fixtures/` is used by strict real-file gates for ProXDR and Motion Photo behavior.

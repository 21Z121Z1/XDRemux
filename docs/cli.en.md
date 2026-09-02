# XDRemux CLI Reference

English | [简体中文](cli.md)

XDRemux has one cross-platform product entry point: the Rust `xdremux` CLI. Input type, Motion Photo / ProXDR routing, and HDR / Gain Map source structure are detected automatically. Standard conversion does not require users to choose a format, device generation, or low-level processing policy.

The Swift and Python implementations remain only as migration-time conformance oracles, Apple platform capability implementations, or research/training tooling. They no longer define new CLI product semantics.

## Commands

| Command | Purpose |
| --- | --- |
| `convert` | Convert one ProXDR photo; supported Motion Photos are automatically converted to Live Photos. |
| `batch` | Discover and convert supported photo assets in batches. |
| `categorize` | Classify assets by asset type and primary capture mode without conversion. |
| `inspect` | Inspect input type and important structure. |
| `validate` | Validate ISO HDR HEIF or Live Photo output. |

Show help with:

```bash
xdremux --help
xdremux convert --help
```

## `convert`

Standard conversion needs no mode option:

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

For an ordinary ProXDR input, omitting `--output` targets the input path and atomically publishes the replacement.

For a supported Motion Photo:

```bash
xdremux convert --input IMG_001.jpg
```

XDRemux detects the Motion Photo automatically and publishes a matching HEIC + MOV Live Photo pair. Motion Photos are never converted in place. When no output is supplied, XDRemux chooses a new name that does not collide with an existing HEIC or MOV companion.

### OPPO Gallery compatibility

When the output must remain compatible with OPPO Gallery, use one product-level switch:

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic \
  --oppo-compatible
```

`--oppo-compatible` means “produce output for OPPO Gallery.” XDRemux selects the internal Gain Map encoding, metadata routing, and platform capabilities from the input and requested product outcome.

OPPO-compatible output currently applies only to ProXDR still images and cannot be combined with Motion Photo → Live Photo conversion. Such a request fails explicitly instead of silently ignoring the option.

## `batch`

Files and directories can be repeated:

```bash
xdremux batch \
  --input-dir photo_dump/ \
  --recursive \
  --output-dir converted/
```

`--input FILE` may also be repeated. Directories are non-recursive by default; add `--recursive` to descend into subdirectories. Hidden files and XDRemux-generated `.xdremux` outputs are not rediscovered.

Common options:

| Option | Purpose |
| --- | --- |
| `--input FILE` | Add one input file; repeatable. |
| `--input-dir DIR` | Add one input directory; repeatable. |
| `--recursive` | Recursively scan input directories. |
| `--output-dir DIR` | Choose the output directory. |
| `--jobs N` | Maximum concurrent conversions; must be greater than zero. |
| `--checkpoint FILE` | Choose a durable checkpoint file. |
| `--resume` | Reuse completed work only when source provenance still matches. |
| `--skip-existing` | Reuse an existing result only when provenance and output identity both match. |
| `--categorize` | Publish converted assets directly into classification folders. |
| `--oppo-compatible` | Request OPPO Gallery compatible output for ProXDR still items. |
| `--json` | Emit a stable machine-readable receipt. |

Batch planning reserves all output paths before the first write, preventing collisions between sources, HEIC outputs, and Live Photo MOV companions. Failures are isolated per item; already-published successful work remains valid.

In a mixed batch with `--oppo-compatible`, ProXDR still images use the requested product intent. Motion Photo items are reported as explicit per-item failures rather than pretending the compatibility policy was applied.

## `categorize`

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` is repeatable and directories are scanned recursively. `--dry-run` plans without publishing, while `--json` emits a machine-readable receipt. A Live Photo HEIC and MOV are handled as one asset.

## `inspect`

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json
```

`inspect` reports facts parsed automatically from the source, such as asset kind, HDR mode, Gain Map data, Motion Photo video range, and presentation timestamp. It is not a conversion-policy configuration surface.

## `validate`

```bash
xdremux validate output.heic
xdremux validate output.heic --json
```

`validate` automatically recognizes and validates either ISO HDR HEIF or a Live Photo pair. It is suitable for independent post-conversion checks in scripts and CI.

## Product intent versus implementation detail

The normal CLI exposes only product intents that change the result a user actually wants. Source recognition, reconstruction algorithms, Gain Map layout, metadata routing, camera-tail handling, and codec/backend selection are engine/runtime decisions derived from the input and available platform capabilities.

For diagnostics, observe those automatic decisions through `inspect`, structured logs, and development tests rather than turning internal policy back into command-line configuration.

## Exit status

Success, help, and version output use `0`. Runtime conversion or validation failures use `1`. Command-line syntax and usage errors use `2`.

## Machine-readable output

`inspect --json`, `batch --json`, `categorize --json`, and `validate --json` provide stable structured output. Human-readable output remains the default.

## Apple feature migration status

Photographic Styles and Portrait are being migrated from independent Swift business implementations into capability contracts owned by the Rust engine. In the final architecture Rust owns policy, orchestration, data models, and the CLI; Apple-native code only invokes platform APIs such as Core Image, Vision, Core ML, and AVFoundation.

Until that migration passes the macOS integration gate, the old Swift Apple implementation remains a migration oracle. Its low-level parameters should not be reintroduced into the Rust CLI.
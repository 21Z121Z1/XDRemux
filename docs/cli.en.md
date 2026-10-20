# XDRemux CLI Reference

English | [简体中文](cli.md)

XDRemux has one product entry point: the Rust `xdremux` CLI. Input type, Motion Photo / ProXDR routing, and HDR / Gain Map source structure are detected automatically. Standard conversion does not require users to choose a format, device generation, or low-level processing policy.

Complete Mac product delivery targets Apple silicon (`arm64`) and macOS 27. The underlying Rust core retains Linux, Windows, and macOS builds and portable tests. A core build or an older compilation floor does not establish availability of Styles SPI or other Apple-native features. Those features require the documented macOS 27 SDK, runtime APIs, hardware, and separate native acceptance checks.

The Swift package is only the Apple platform capability adapter, and the remaining Python package is research/training tooling. Neither provides a second XDRemux runtime; they no longer define new CLI product semantics or a source of CLI product policy.

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

## Terminal language

Use `xdremux --lang en --help` or `xdremux --lang zh-CN --help` for English or
Simplified Chinese terminal presentation. The global option also works after a
subcommand. Selection order is `--lang`, `XDREMUX_LANG`, `LC_ALL`, `LC_MESSAGES`,
`LANG`, then English. Empty environment values are ignored. Supported `zh`
locales, such as `zh_CN.UTF-8`, select Simplified Chinese; `C` and `POSIX` select
English. An unsupported environment locale falls back to English. An unsupported
explicit `--lang` is a usage error.

JSON documents, command and option names, enum values, exit codes, and input
path arguments do not change with language. Human help, labels, and CLI-owned
error context are translated. Clap's detailed parser diagnostics and lower-level
library/native errors may remain English.

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

### Apple Portrait on macOS

Use the source-native profile when private Apple segmentation is not required:

```bash
xdremux convert --input portrait.heic --output portrait_apple.heic --apple-portrait-oppo
xdremux batch --input-dir portraits/ --output-dir converted/ --apple-portrait-oppo --jobs 2
```

This profile preserves OPPO depth as disparity, focus metadata, aperture, Rust-generated REND, and the ISO Gain Map. It adds valid OPPO portrait and hair masks when available. Empty or unusable masks remain absent. It does not generate skin, teeth, or glasses masks and makes no Vision requests.

**Portrait effects are degraded compared with the complete profile.** Subject and hair boundaries can be less accurate. Semantic relighting and related edits can be limited. This mode uses public ImageIO and Core Image APIs on macOS; it is not a Linux/Windows Portrait writer. Photos editing behavior remains dependent on the consumer and OS version.

Use `--apple-portrait` for the existing complete resource set with Vision semantics, or `--apple-styles` for Photographic Styles. These options and `--apple-portrait-oppo` are mutually exclusive and cannot be combined with `--oppo-compatible`. Inputs without the required OPPO depth/source data fail before publication. The CLI does not silently switch profiles after a failure.

## `batch`

Files and directories can be repeated:

```bash
xdremux batch \
  --input-dir photo_dump/ \
  --recursive \
  --output-dir converted/
```

`--input FILE` may also be repeated. Directories are non-recursive by default; add `--recursive` to descend into subdirectories. Hidden files and XDRemux-generated `.xdremux` outputs are not rediscovered.

Each converted Live Photo batch success includes `omitted_resource_count` in
JSON. Human output reports nonzero omissions. A reused checkpoint has no new
conversion report, so this field is `null` rather than an assumed zero.
Existing batch outputs remain unchanged if a competing writer publishes after
planning; publication refuses to replace that file.

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
| `--apple-portrait` | Request the complete Portrait resource set on macOS. |
| `--apple-portrait-oppo` | Use OPPO depth and available masks with reduced effects on macOS. |
| `--apple-styles` | Generate Photographic Styles and inferred Texture Styles on macOS 27. |
| `--apple-cinematic` | Infer pending Cinematic metadata for ordinary video on macOS 27. |
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

## Cinematic video intent (macOS)

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
xdremux batch --input-dir videos/ --output-dir converted/ --apple-cinematic --jobs 2
```

`--apple-cinematic` activates this feature on `convert` and `batch`, like `--apple-styles` and `--apple-portrait`. It is mutually exclusive with the image feature and OPPO compatibility flags. The selected path generates new Cinematic metadata from decoded video frames. It copies the compressed video and audio samples into a new MOV. The output must use the `.mov` extension and must not exist. If `convert --output` is omitted, the command creates `SOURCE.xdremux.mov` beside the input. The input is never replaced.

Before publication, the Apple adapter requires public `CNAssetInfo` to report `needsPreprocessing`, resource status `ready`, and `isPreprocessed == false`. The MOV contains no materialized raster disparity track. This state is the acceptance target; it does not establish Photos editing or rendering quality.

The current implementation requires macOS 27, a macOS 27 SDK build of the adapter, the corresponding Apple private framework methods, and supported inference and tracking hardware/resources. It fails when a required SDK, OS, method, or resource is unavailable. Import the pending MOV into Photos and enable Cinematic to let Photos process and render its resources.

This command is experimental. Successful pending-state recognition does not guarantee native processing for every input. On macOS 27.0.1, an independent 1080p H.264 input processed and rendered successfully; a 640×360 H.264 input reached the pending state but Apple's preprocessing rejected it as unsupported. The command preserves encoded media without resizing or transcoding. See the [Cinematic contract](cinematic.en.md) for the tested scope and consumer checks.

For this intent, batch directory discovery selects MOV, MP4, M4V, AVI, and MKV candidates. AVFoundation decides whether their media can be decoded. Generated `.xdremux` files are not rediscovered. Output planning reserves unique `.mov` names; receipts use kind `cinematic`. Live Photo checkpoint/resume and photo categorization options do not apply to this video intent.

`--focal-length-35mm MM` defaults to 24. `--aperture F` defaults to 2.8. Supplying either parameter requires `--apple-cinematic`. These are assumed capture values for input without camera geometry. They are not recovered physical camera measurements. See the [Cinematic envelope contract](cinematic.en.md) for the native pipeline and verification procedure.

## Product intent versus implementation detail

The normal CLI exposes only product intents that change the result a user actually wants. Source recognition, reconstruction algorithms, Gain Map layout, metadata routing, camera-tail handling, and codec/backend selection are engine/runtime decisions derived from the input and available platform capabilities.

For diagnostics, observe those automatic decisions through `inspect`, structured logs, and development tests rather than turning internal policy back into command-line configuration.

With `validate --apple-styles`, Rust also requires native Photographic Styles and Texture readback on macOS 27. The app uses this check before it skips an existing Styles output. A plain HDR file does not pass this feature check.

## Exit status

Success, help, and version output use `0`. Runtime conversion or validation failures use `1`. Command-line syntax and usage errors use `2`.

## Machine-readable output

`inspect --json`, `batch --json`, `categorize --json`, and `validate --json` provide stable structured output. Human-readable output remains the default.

## Apple feature ownership

Photographic Styles and Portrait are requested through the `convert` and `batch` intents. Rust owns policy, orchestration, data models, metadata, assembly, validation, and publication. `xdremux-apple-adapter` only invokes platform APIs such as ImageIO, Vision, Core Image, Core ML, and VideoToolbox and returns factual observations or primitive-operation results.

The Rust-driven macOS gates prove structural output and the tested native consumer facts. Visual equivalence and interactive Photos behavior remain separate device-level claims and must not be inferred from a successful parser or adapter probe.

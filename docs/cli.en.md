# XDRemux CLI reference

English | [简体中文](cli.md)

XDRemux has one product entry point: the Rust `xdremux` CLI. Source type, ProXDR or Motion Photo routing, and HDR structure are detected automatically. Standard conversion does not require a device-generation, format, or codec switch.

Complete Mac product delivery targets Apple silicon (`arm64`) and macOS 27. The Rust core retains Linux, Windows, and macOS builds and portable tests. A portable build does not establish availability of Styles SPI or other Apple features. Those features need the documented SDK, runtime APIs, hardware, and native checks.

The Swift package provides the Apple adapter. Python tools support research and training. They no longer define new CLI product semantics or a separate product policy.

## Commands

| Command | Input and result |
| --- | --- |
| `convert` | One supported ProXDR still or Motion Photo. Supported Motion Photos are automatically converted to Live Photos. `--apple-cinematic` selects ordinary video instead. |
| `batch` | Repeated files or directories, with one result for each planned item. |
| `categorize` | Photo assets copied into folders by asset type and primary capture mode. No conversion. |
| `inspect` | Parsed source facts. No conversion. |
| `validate` | Supported ISO HDR HEIF, a Live Photo member, or a PVT directory. No conversion. |

```bash
xdremux --help
xdremux convert --help
```

The option definitions are in [the CLI source](../crates/xdremux-cli/src/lib.rs). Classification and validation use their own [categorize](../crates/xdremux-cli/src/categorize.rs) and [validate](../crates/xdremux-cli/src/validate.rs) modules.

## Terminal language

`--lang en` selects English. `--lang zh-CN` selects Simplified Chinese. The global option can occur before or after a subcommand.

Language selection uses the first applicable value in this order:

1. `--lang`
2. `XDREMUX_LANG`
3. `LC_ALL`
4. `LC_MESSAGES`
5. `LANG`
6. English

Empty environment values are ignored. Supported `zh` locales, including `zh_CN.UTF-8`, select Simplified Chinese. `C` and `POSIX` select English. Unsupported environment locales fall back to English. An unsupported explicit `--lang` value is a usage error.

Language changes human help, labels, and error context owned by the CLI. It does not change JSON, command names, option names, enum values, paths, or exit codes. Clap parser details and lower-level library or native errors may remain English.

## `convert`

`--input INPUT` is required. `--output OUTPUT` is optional, but its default depends on the source and requested feature.

| Source or feature | Without `--output` | Source preservation |
| --- | --- | --- |
| ProXDR still, including Styles or Portrait | The input path is the destination. | Successful conversion replaces the input through atomic publication. |
| Motion Photo with default pair output | A new HEIC basename beside the source, with a matching MOV. | The source is not replaced. The selected names must not collide with existing pair members. |
| Motion Photo with PVT output | A new `.pvt` directory beside the source. | The source is not replaced. |
| `--apple-cinematic` | `SOURCE.xdremux.mov` beside the source. | The source is not replaced. An existing destination is rejected. |

> [!CAUTION]
> To retain a ProXDR original, specify a separate `--output` path. Omitting this option replaces the source after successful conversion.

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux convert --input IMG_001.jpg
```

The second example requires a supported Motion Photo. The file extension alone does not establish support.

### Product options

The following options are available to both `convert` and `batch`.

| Option | Requirement and effect |
| --- | --- |
| `--live-photo-output pair\|pvt` | Default `pair`. Selects a Live Photo file pair or PVT directory. Other assets keep their normal format. |
| `--oppo-compatible` | Requests OPPO Gallery output for a ProXDR still. Motion Photo items reject this request. |
| `--apple-portrait` | Requests the complete Apple Portrait resource set, including Vision semantics. |
| `--apple-portrait-oppo` | Uses OPPO depth and available masks. Requires macOS ImageIO and Core Image. Effects are reduced. |
| `--apple-styles` | Generates Photographic Styles and inferred Texture Styles on macOS 27. Requires supported ProXDR source data. |
| `--apple-cinematic` | Generates pending Cinematic metadata for ordinary video on macOS 27. Experimental. |
| `--focal-length-35mm MM` | Requires `--apple-cinematic`. Overrides automatic recorded-metadata or GeoCalib focal selection. |
| `--cinematic-camera-estimate PATH` | `convert` only. Supplies explicit source-bound camera observations below explicit focal input and above automatic selection. |
| `--aperture F` | Requires `--apple-cinematic`. Defaults to `2.8`. Supplies an assumed capture aperture. |

`--oppo-compatible`, `--apple-portrait`, `--apple-portrait-oppo`, `--apple-styles`, and `--apple-cinematic` are mutually exclusive. A failed feature does not silently switch to another profile. Required source data or native capabilities must exist before the program publishes output.

### PVT directory output

```bash
xdremux convert --input IMG_001.heic --output capture.pvt --live-photo-output pvt
xdremux validate capture.pvt --json
```

This example requires a supported Motion Photo. `capture.pvt` contains `capture.heic`, `capture.mov`, and `metadata.plist`. The plist stores `PFVideoComplementMetadataVersionKey` as the string `"1"`.

Packaging preserves the generated pair's bytes, identifier, and cover time. It does not encode the still or remux the MOV again. The preceding conversion can still add pairing metadata or encode a still when required. Omitted vendor resources remain reported, so PVT does not make conversion lossless.

An explicit package destination must end in `.pvt`. Existing files, directories, and symbolic links are preserved and rejected. Supported macOS and Linux filesystems use an atomic, exclusive directory rename. Unsupported platforms or filesystems fail without an overwriting fallback.

Failed publication may leave a complete staged package and report its path for recovery. This is not a guarantee of durability after power loss. A ZIP file renamed to `.pvt` is rejected.

PVT selection affects Motion Photo items only. A mixed batch retains normal HDR output for stills. A still cannot write disguised HEIF into an explicit `.pvt` destination. Batch JSON lists the package directory as one output.

Reuse requires matching source provenance and the requested container. An old flat pair or an unrelated valid package cannot satisfy a PVT request. Existing PVT packages and their members are not input assets for `convert`, `batch`, or `categorize`.

Directory discovery skips PVT directories. Output and checkpoint writers reject destinations inside an existing package, including paths through aliases. The app exposes the same choice and delegates creation to Rust. Structural validity alone does not permit the app to adopt an existing package.

Package generation and validation do not establish Photos import or dynamic wallpaper support. See [Live Photo integration](liblivephoto-integration.en.md).

### OPPO Gallery output

```bash
xdremux convert --input IMG_001.heic --output IMG_001_oppo.heic --oppo-compatible
```

The engine selects Gain Map encoding, metadata routing, and platform operations from the source and requested output. Motion Photo conversion fails if `--oppo-compatible` is set. In a mixed batch, those failures are reported per Motion Photo item.

### Apple Portrait

```bash
xdremux convert --input portrait.heic --output portrait_apple.heic --apple-portrait-oppo
xdremux batch --input-dir portraits/ --output-dir converted/ --apple-portrait-oppo --jobs 2
```

The OPPO profile converts source depth to disparity. It retains focus metadata, aperture, Rust-generated REND, and the ISO Gain Map. It adds valid OPPO person and hair masks when available. Empty or unusable masks remain absent.

This profile generates no skin, teeth, or glasses masks and makes no Vision requests. Subject and hair boundaries can be less accurate than with the complete profile. Semantic relighting and related edits can be limited.

The profile still requires public ImageIO and Core Image APIs on macOS. It is not a Linux or Windows Portrait writer. Photos editing depends on the consumer and OS version.

`--apple-portrait` requests the complete resource set with Vision semantics. `--apple-styles` requests Photographic Styles. These image features require supported ProXDR stills and the necessary source resources. Missing required OPPO depth or source data causes failure before publication.

## `batch`

```bash
xdremux batch --input-dir photo_dump/ --recursive --output-dir converted/
```

At least one `--input` or `--input-dir` is required. Both can occur more than once. Directory scanning is non-recursive unless `--recursive` is set. Scanning ignores symbolic links, hidden entries, PVT directories, and generated `.xdremux` outputs.

Normal directory discovery selects HEIC, HEIF, JPG, and JPEG candidates. `--apple-cinematic` selects MOV, MP4, M4V, AVI, and MKV candidates instead. Candidate discovery does not prove that a file can be decoded.

| Option | Default and behavior |
| --- | --- |
| `--input FILE` | Adds one explicit file. Repeatable. |
| `--input-dir DIR` | Adds one directory. Repeatable. |
| `--recursive` | Disabled by default. Descends into input directories. |
| `--output-dir DIR` | Without this option, generated outputs are beside their sources with an `.xdremux` suffix. |
| `--jobs N` | Positive integer. Defaults to available parallelism, capped at `4`, with fallback `1`. |
| `--checkpoint FILE` | Selects the checkpoint base path. The stored filename appends `.motion-photo`. |
| `--resume` | Reuses completed Live Photo work only when provenance and output identity match, then retries remaining items. |
| `--skip-existing` | Applies the same provenance checks before reusing existing Live Photo output. |
| `--categorize` | Publishes converted assets directly into classification folders. |
| `--json` | Writes one structured receipt instead of human output. |

The [product options](#product-options) also apply. `--resume` and `--skip-existing` require `--output-dir` or `--checkpoint`. Without an explicit checkpoint, `--output-dir` stores `.xdremux-motion-photo-checkpoint.jsonl`.

Checkpoint reuse is a Live Photo feature. It is not a general promise to skip any existing HDR or video output. `--apple-cinematic` rejects `--resume`, `--skip-existing`, `--checkpoint`, and `--categorize`.

Planning reserves source and output paths before writing starts. Reservations include the MOV companion of each Live Photo. If another writer creates an output after planning, publication refuses to replace that output. Each item has its own success or failure result. Earlier successful outputs remain valid after another item fails.

Each newly converted Live Photo success reports `omitted_resource_count` in JSON. Human output reports a nonzero count. Reuse produces no new conversion report, so the field is `null`, not an assumed zero.

## `categorize`

```bash
xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run --json
```

`--input PATH` is repeatable and required at least once. `--output-dir DIR` is required. Input directories are scanned recursively. `--dry-run` reports the plan without publishing files. `--json` produces a structured receipt.

The command copies assets into folders by asset type and primary capture mode. A validated Live Photo still and MOV are one asset. Explicit symbolic-link inputs are rejected. Directory discovery skips symbolic links and PVT directories.

## `inspect`

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json
```

The positional `INPUT` is required. `--json` selects structured output. Reported facts include asset kind, HDR mode, Gain Map data, Motion Photo video range, and presentation timestamp when available.

Inspection does not change conversion policy or guarantee that a source can be converted.

## `validate`

```bash
xdremux validate output.heic
xdremux validate output.mov --json
xdremux validate capture.pvt --json
```

The positional `INPUT` is required. Plain validation recognizes supported ISO HDR HEIF, either member of a Live Photo pair, or a three-member PVT directory. It checks the corresponding media, pairing, and timing contract.

PVT validation also checks the XML plist's string version and unambiguous regular member files. It rejects missing, nested, symbolic-link, duplicate, or malformed members without changing them. This check is structural validation, not Photos import.

`validate STYLES_OUTPUT --apple-styles` additionally requires native Photographic Styles and Texture readback on macOS 27. The app uses this check before it skips an existing Styles output. A plain HDR file fails the feature check.

## Cinematic video on macOS

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
xdremux batch --input-dir videos/ --output-dir converted/ --apple-cinematic --jobs 2
```

The command generates new metadata from decoded video frames. It copies compressed video and audio samples into a new MOV. The destination must have a `.mov` extension and must not exist. The source is never replaced.

Before publication, public `CNAssetInfo` must report `needsPreprocessing`, resource status `ready`, and `isPreprocessed == false`. The MOV has no materialized raster disparity track. This pending state is the conversion acceptance target. It does not establish Photos editing or rendering quality.

The adapter requires macOS 27, a macOS 27 SDK build, the relevant private framework methods, and supported inference and tracking resources. Missing SDK, OS, methods, or resources cause failure.

Import the pending MOV into Photos and enable Cinematic for native processing. This experimental path does not guarantee that every recognized pending asset can be processed. On macOS 27.0.1, an independent 1080p H.264 sample processed and rendered. A 640×360 H.264 sample reached the pending state, but Apple preprocessing rejected it as unsupported.

The command preserves encoded media without resizing or transcoding. AVFoundation decides whether discovered video candidates can be decoded. Batch results use kind `cinematic` and reserve unique MOV names.

Both entry points choose focal input automatically: explicit `--focal-length-35mm MM`, then a fixed recorded 35mm-equivalent tag read by ExifTool, then the installed GeoCalib helper when the tag is absent or unknown. Physical focal length alone is insufficient. There is no default 24 mm substitution. Missing dependencies, conflicting metadata or rejected model observations fail before output publication. `--aperture F` remains a separate assumption with default 2.8. Either explicit parameter requires `--apple-cinematic`. See the [Cinematic envelope contract](cinematic.en.md) for admitted tags, limits and native verification.


`convert --cinematic-camera-estimate camera.json` remains an explicit observation override above automatic selection. An explicit `--focal-length-35mm` takes priority. Normal use needs no JSON argument, and `batch` resolves each source independently. The [GeoCalib model adapter](../model_adapters/geocalib/README.en.md) requires one-time local setup and currently supports fixed-view SDR video. Rust owns admission and focal selection. It does not recover capture aperture, focus distance or unknown EIS crop, and low model uncertainty does not establish physical accuracy.

## Exit status and JSON

| Exit code | Meaning |
| --- | --- |
| `0` | Success, help, or version output. |
| `1` | Runtime conversion, validation, or output failure. A batch can have successful items and still return `1`. |
| `2` | Syntax, usage, input discovery, or planning error. |

`inspect`, `batch`, `categorize`, and `validate` accept `--json`. `convert` does not. Human output is the default. Language selection leaves the structured schemas unchanged.

## Product policy and Apple adapter

The engine and runtime select source reconstruction, Gain Map layout, metadata routing, camera-tail handling, and codecs. Use `inspect`, structured results, and development checks to observe these decisions.

Rust owns feature policy, data models, metadata assembly, validation, and publication. `xdremux-apple-adapter` calls ImageIO, Vision, Core Image, Core ML, VideoToolbox, and other required Apple APIs. It returns operation results and observed facts.

A successful parser or adapter check proves only the contract that it tested. Visual equivalence and interactive Photos editing require separate evidence from the relevant environment.

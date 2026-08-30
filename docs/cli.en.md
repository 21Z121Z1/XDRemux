# XDRemux CLI Reference

English | [简体中文](cli.md)

XDRemux has a Swift CLI and a Python CLI. The two implementations share the main HDR, Motion Photo, and classification goals, but their command details are not identical.

## Swift CLI

The Swift executable is `xdremux`.

```bash
swift build
swift run xdremux --help
```

A release build is recommended for Photographic Styles work:

```bash
swift build -c release
.build/release/xdremux --help
```

### Commands

| Command | Function |
| --- | --- |
| `convert` | Convert one HDR photo or automatically convert one Motion Photo. |
| `batch` | Recursively process a directory. The default path can contain normal HDR photos and Motion Photos. |
| `categorize` | Copy photo assets into classification directories without conversion. |
| `validate-apple` | Validate Photographic Styles output and print JSON. |
| `validate-portrait` | Validate Apple Portrait output and print JSON. |
| `portrait-self-test` | Run the portrait core self-test and print JSON. |

### `convert`

For a normal ProXDR HEIC or HEIF:

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

If `--output` is absent, normal HDR conversion targets the input path. The normal HDR path can therefore replace the input file.

For a supported Motion Photo:

```bash
xdremux convert --input IMG_001.jpg
```

Motion Photo detection is automatic for supported `.jpg`, `.jpeg`, `.heic`, and `.heif` inputs.

A Motion Photo is never converted in place. Without `--output`, XDRemux reserves a new HEIC name next to the source and creates the companion MOV. If `IMG_001.heic` or `IMG_001.mov` already exists, the implicit output uses the next available basename such as `IMG_001 (2)`.

If you set `--output` for a Motion Photo, the still output must use `.heic` or `.heif`. XDRemux fails if the requested HEIC/HEIF or companion MOV already exists.

Plain Motion Photo conversion cannot use Apple Portrait, Photographic Styles, or OPPO-compatible output in the same pass. A separate opt-in path supports Motion Photo + Photographic Styles for single-file `convert`; see the [Apple features guide](apple-features.en.md).

### `batch`

```bash
xdremux batch --input-dir photo_dump/ --output-dir converted/
```

The Swift batch command scans recursively.

Without an explicit `--glob`, the CLI also discovers supported JPEG/JPG and HEIC/HEIF Motion Photos and routes them to the Live Photo converter. It keeps generated Live Photo stills out of the normal ProXDR pass.

An explicit `--glob` keeps the normal batch parser contract. Do not use an explicit HEIC-only glob when you expect automatic JPEG Motion Photo discovery.

Swift Motion Photo batch state is durable. The default values are:

- `--resume`: on;
- `--skip-existing`: on;
- `--jobs`: `min(cpu, 4)`;
- checkpoint path: a hidden JSONL file under the output directory unless `--checkpoint` sets another path.

An existing Live Photo pair is reused only when saved source provenance and the pair identity match. A valid pair with unknown lineage is not silently assigned to a different source.

### `categorize`

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` is repeatable. `--dry-run` prints the plan without copying files.

Classification keeps a validated Live Photo HEIC and MOV together as one asset. The directory projection first separates static photos and Live Photos, then uses the primary capture mode.

### Common Swift conversion options

| Option | Default | Function |
| --- | --- | --- |
| `--family auto|x6|x7` | `auto` | Select the source ProXDR family. |
| `--oppo-compatible` | off | Request automatic OPPO Gallery compatibility. |
| `--oppo-compat [mode]` | off | Select a fine-grained OPPO compatibility mode. A bare flag means `on`. |
| `--no-oppo-compat` | off | Force standard non-OPPO-compatible output. |
| `--oppo-camera-tail <mode>` | `preserve-without-private-hdr` | Select the OPPO private-tail policy. |
| `--discard-portrait-data` | off | Remove bulky OPPO portrait/depth editing resources when the selected tail policy permits it. |
| `--input-processing system|system-decoded|hybrid|passthrough` | `hybrid` | Select the HDR input-processing branch. |
| `--tmap-format imageio|strict` | `imageio` | Select the tone-map metadata writer. |
| `--debug-dir <dir>` | none | Keep diagnostic artifacts. |
| `--apple-photographic-styles` | off | Enable Photographic Styles generation. `--apple-styles` is a legacy spelling. |
| `--apple-portrait` | off | Enable Apple Portrait generation. |
| `--apple-styles-raw-dng <file>` | none | Supply a matching RAW DNG for Photographic Styles analysis. |
| `--apple-style-data-producer <mode>` | `constrained-solver` when Styles is enabled | Select `constrained-solver`, `learn-node`, or `identity-fallback`. |

`--apple-styles-raw-dng` and `--apple-style-data-producer` require `--apple-photographic-styles`.

Apple features and OPPO-compatible output are mutually exclusive.

### `--oppo-camera-tail`

The parser accepts these values:

| Value | Intent |
| --- | --- |
| `off` | Do not append the OPPO private tail. |
| `watermark` | Keep watermark, master-mode presets, and capture parameters. |
| `compact` | Keep the watermark data and a compact portrait/depth tail. |
| `preserve` | Preserve the complete tail. |
| `preserve-without-portrait` | Remove depth, masks, meshes, and restore-original resources. |
| `preserve-without-portrait-or-private-hdr` | Remove portrait resources and private HDR entries. |
| `preserve-without-private-uhdr` | Remove private UHDR Gain Map entries. |
| `preserve-without-private-hdr` | Default non-OPPO-compatible policy; remove private HDR entries and keep other supported vendor data. |
| `preserve-no-uhdr` | Keep bytes but disable private UHDR manifest keys in place. |
| `preserve-no-hdr` | Keep bytes but disable private HDR manifest keys in place. |

### Swift exit behavior

Normal success and help output use exit status `0`.

Runtime conversion failures use exit status `1`.

Swift Argument Parser handles its own parser-level usage errors. Motion Photo pre-routing errors are caught by the XDRemux entry point and use exit status `1`. Do not treat all invalid-command failures as one error class when scripting the CLI.

## Python CLI

The Python executable is `xdremux-py`. Python 3.11 or newer is required.

```bash
pip install -e .
xdremux-py --help
```

The repository-local form is:

```bash
python3 -m xdremux_py --help
```

### Python commands

The Python CLI provides `convert`, `batch`, and `categorize`.

It supports standard HDR conversion, Motion Photo to Live Photo conversion, and classification. It does not generate Apple Photographic Styles or Apple Portrait data.

### Python `convert`

```bash
xdremux-py convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux-py convert --input IMG_001.jpg
```

For normal ProXDR input, omitting `--output` targets the input file.

For Motion Photo input, omitting `--output` always creates a new HEIC + MOV pair and preserves the source. An explicit Motion Photo output fails if the target pair already exists.

Python Motion Photo conversion uses the default conversion configuration only. Do not combine it with `--oppo-compatible`, `--reencode`, or `--debug-dir`.

### Python `batch`

Without `--glob`, Python batch discovery is non-recursive and examines files directly inside `--input-dir`. It recognizes HEIC/HEIF inputs and supported JPEG/JPG Motion Photos.

`--skip-existing` and `--resume` are opt-in in the Python CLI. Both use durable source provenance before a Live Photo pair can be reused.

`--checkpoint` sets the Motion Photo state file.

### Python `categorize`

```bash
python3 -m xdremux_py categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` is repeatable. `--jobs` defaults to `min(cpu, 4)`. `--dry-run` does not copy files.

### Python exit behavior

Success uses exit status `0`.

Runtime command failures use exit status `1`.

Python `argparse` uses exit status `2` for parser-level usage errors.

## Machine-readable output

The Swift `validate-apple`, `validate-portrait`, and `portrait-self-test` commands write JSON to stdout.

The normal Swift and Python conversion commands write human-readable progress. They do not provide a general JSON event-stream mode.

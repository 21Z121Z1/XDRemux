# XDRemux CLI Reference

English | [简体中文](cli.md)

This document covers the `xdremux` command-line tool. `xdremux --help` prints the same material.

## Build and run

```bash
swift build
swift run xdremux --help
```

You can also invoke the built binary directly:

```bash
.build/debug/xdremux convert --input IMG_001.heic
```

## Commands

| Command | What it does |
| --- | --- |
| `convert` | Convert one photo |
| `batch` | Convert a directory recursively |
| `categorize` | Sort files by shooting mode without converting anything |
| `validate-apple` | Inspect a file's Apple Photographic Styles output; prints JSON to stdout |
| `validate-portrait` | Inspect a file's Apple portrait output; prints JSON to stdout |
| `portrait-self-test` | Run the portrait pipeline self-test; prints JSON to stdout |

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux batch --input-dir ~/Pictures/ProXDR --output-dir ~/Pictures/HDR
xdremux categorize --input ~/Pictures/ProXDR --output-dir ~/Pictures/Sorted
```

## Where the results go

- `convert` **overwrites the input file** when `--output` is omitted.
- `batch` writes back into the input directory when `--output-dir` is omitted.
- `batch --categorize` files results under Chinese shooting-mode folders (`人像`, `夜景`, `大师模式`, …). Photos whose mode cannot be read stay in the output root.
- `categorize` only copies HEIC/HEIF/JPEG files into those folders. It never modifies or converts anything.

## Options

### Conversion options (`convert` and `batch`)

| Option | Default | Description |
| --- | --- | --- |
| `--input <file>` | required for `convert` | Input photo, HEIC or portrait JPEG |
| `--output <file>` | overwrite the input | Output photo |
| `--oppo-compatible` | off | Write a 4:2:0 gain map OPPO Gallery can display and keep the complete OPPO private tail. Without it the output is standard ISO HDR and the gain map keeps its source channel structure, which may be 4:4:4. A gain map that is already 4:2:0 cannot be upgraded — the discarded chroma is unrecoverable. |
| `--discard-portrait-data` | off | Drop bulky depth and re-edit resources. Watermark, master-mode, and other non-HDR vendor data are still kept. |
| `--oppo-camera-tail <mode>` | `preserve-without-private-hdr` | Which parts of the OPPO camera tail to keep; see the table below. |
| `--family auto\|x6\|x7` | `auto` | Which ProXDR layout the source uses |
| `--debug-dir <dir>` | not written | Keep this run's intermediate artifacts for inspection |

### Batch options (`batch`)

| Option | Default | Description |
| --- | --- | --- |
| `--input-dir <dir>` | required | Input directory, scanned recursively |
| `--output-dir <dir>` | the input directory | Output directory |
| `--glob <pattern>` | `*.heic` | Which files to pick up |
| `--jobs <n>` | `min(cpu, 4)` | How many files to convert at once |
| `--categorize` | off | File results under shooting-mode folders |
| `--resume` / `--no-resume` | `--resume` | Continue from the previous run's progress |
| `--skip-existing` / `--no-skip-existing` | `--skip-existing` | Skip a file whose output already matches the current settings |
| `--checkpoint <file>` | a hidden JSONL file under the output directory | Where progress is recorded |

### Categorize options (`categorize`)

| Option | Default | Description |
| --- | --- | --- |
| `--input <file-or-dir>` | required, repeatable | What to sort |
| `--output-dir <dir>` | required | Root of the sorted folders |
| `--jobs <n>` | `min(cpu, 4)` | Concurrency |
| `--dry-run` | off | Print the plan without copying anything |

### Apple features (macOS only, research features)

| Option | Default | Description |
| --- | --- | --- |
| `--apple-photographic-styles` | off | Generate Apple Photographic Styles data from the photo itself, with no Apple donor photo. `--apple-styles` is a legacy spelling. |
| `--apple-portrait` | off | Generate Apple portrait data. Needs a photo that carries `rear.depth`, `rear.depth.config`, and `src.image`. |
| `--apple-styles-raw-dng <file>` | none | Pair one matching OPPO RAW MAX DNG with the input. A mismatched or differently oriented DNG is rejected rather than used. |
| `--apple-style-data-producer <mode>` | `constrained-solver` | One of `constrained-solver`, `learn-node`, `identity-fallback`. The last two are diagnostic controls. |

The two features are independent and can be enabled together; in a combined run a non-portrait photo still gets styles output. Apple output and `--oppo-compatible` are mutually exclusive.

These features are **not accepted as production Photos output**. See the [Apple features guide](apple-features.en.md) for exactly what has and has not been proven.

### Diagnostic options

Needed only when investigating a problem; ordinary use can ignore them.

| Option | Default | Description |
| --- | --- | --- |
| `--input-processing system\|system-decoded\|hybrid\|passthrough` | `hybrid` | How the base image and gain map are rebuilt |
| `--tmap-format imageio\|strict` | `imageio` | `strict` writes the 145-byte ISO form, which breaks Gallery Exif parsing and editing on Find X9 Ultra |
| `--oppo-compat <mode>` | `off` | Finer-grained control over the HDR routing flags: `auto`, `iso`, `iso-no-local`, `iso-graph`, `on`, `tail`, `off`. `--no-oppo-compat` means `off`. |

### `--oppo-camera-tail` values

| Value | Description |
| --- | --- |
| `off` | Append no OPPO camera tail at all |
| `watermark` | Keep only watermark, master-mode presets, and capture parameters |
| `compact` | Watermark plus a compact portrait/depth tail |
| `preserve` | Copy the complete tail byte for byte |
| `preserve-without-portrait` | Keep everything except depth, masks, meshes, and the restore-original image |
| `preserve-without-portrait-or-private-hdr` | The same, plus removing every private HDR entry |
| `preserve-without-private-uhdr` | Physically remove only `local.uhdr.gainmap.data/info` |
| `preserve-without-private-hdr` | **Default**: physically remove every private HDR entry, keeping portrait, watermark, master mode, and the rest |
| `preserve-no-uhdr` | Keep every byte; disable private UHDR by renaming its manifest keys in place |
| `preserve-no-hdr` | Keep every byte; disable every private HDR manifest key in place |

## Output and exit codes

The CLI prints human-readable text: progress on stdout, errors on stderr. There is no JSON event stream, and no `--quiet`, `--verbose`, `--format`, or `--language` option.

| Exit code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Any error — bad arguments, unsupported input, a failed conversion, or a batch with failures |

## Re-running a batch

`batch` writes a hidden JSONL progress file under the output directory and deletes it only when the whole batch finishes with no failures. Running the same command again:

1. Skips outputs that already match the current settings (`--skip-existing`, on by default).
2. Retries the files that failed (`--resume`, on by default).
3. Does not re-scan the shooting-mode folders `--categorize` wrote, so repeated runs are idempotent.

## Common errors

| Message | What it means |
| --- | --- |
| `not a ProXDR photo` | The photo carries no OPPO Local HDR data. It may be an ordinary HEIC, or ProXDR was off when it was taken. |
| `already converted` | The file already carries an ISO 21496-1 gain map; converting it again would change nothing. |
| `not an OPPO portrait photo` | The depth data `--apple-portrait` needs is not in this photo. |
| `N file(s) failed to convert` | The batch had failures. Running the same command again retries only those files. |

## Python CLI

The Python version does HDR conversion only — no Apple Photographic Styles and no Apple portrait.

```bash
pip install -e .
xdremux-py convert --input IMG_001.heic
xdremux-py convert --oppo-compatible --input IMG_001.heic
```

Without installing, run the same commands from the repository root with `python3 -m xdremux_py` or `python3 xdremux/python/XDRemux.py`.

The implementation is the `xdremux_py/` package at the repository root: `cli.py` handles arguments and output, `pipeline.py` performs conversion, and `commands.py` holds the parsed command models. `xdremux/python/XDRemux.py` is a compatibility entry point that forwards to the package.

It needs Python 3.11 or newer. Prefer the Swift CLI for new work and automation.

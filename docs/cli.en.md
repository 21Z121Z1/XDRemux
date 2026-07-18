# XDRemux CLI Reference

English | [简体中文](cli.md)

This document covers the public `xdremux` command. Experimental controls, validators, and internal diagnostics are documented in the [development guide](development.en.md).

## Build and run

```bash
swift build
swift run xdremux --help
```

The legacy script entry point remains available while existing automation migrates:

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

Both entry points invoke the same package executable and have the same commands, defaults, and exit codes.

## Commands

```text
xdremux convert --input <file> [--output <file>] [options]
xdremux batch --input-dir <directory> [--output-dir <directory>] [options]
```

`convert` processes one file. `batch` recursively processes a directory and preserves relative input paths under the output directory.

## Public options

| Option | Scope | Description |
| --- | --- | --- |
| `--input <file>` | `convert` | Input photo |
| `--output <file>` | `convert` | Output photo; omitting it overwrites the input |
| `--input-dir <directory>` | `batch` | Input directory |
| `--output-dir <directory>` | `batch` | Output directory; omitting it writes in place |
| `--glob <pattern>` | `batch` | File matching pattern |
| `--jobs <count>` | `batch` | Maximum concurrent conversions |
| `--overwrite` | Both | Regenerate even when an existing output is valid |
| `--discard-portrait-data` | Both | Do not preserve original vendor portrait-editing data |
| `--oppo-compatible` | Both | Produce OPPO Gallery-compatible output |
| `--apple-photographic-styles` | Both | Generate Apple Photographic Styles resources |
| `--apple-portrait` | Both | Generate Apple Portrait resources |
| `--quiet` | Both | Show only errors and the final result |
| `--verbose` | Both | Add per-file results, major paths, and skip reasons |
| `--debug` | Both | Add internal configuration, temporary paths, and full diagnostics |
| `--format text\|json\|jsonl` | Both | Select human-readable or machine output |
| `--language auto\|zh-Hans\|en` | Both | Select the language for human-readable text |

Apple Photographic Styles and Apple Portrait can be combined. `--oppo-compatible` conflicts with either Apple mode and is rejected before conversion begins.

## Output modes

Default text mode shows the task overview, current progress, warnings, failures, and the final summary. Successful batch jobs do not print one new line per file.

| Mode | Output |
| --- | --- |
| Default | Overview, progress, warnings, failures, and summary |
| `--quiet` | Errors and final result |
| `--verbose` | Default output plus per-file completion, skip reasons, and warning codes |
| `--debug` | Verbose output plus internal configuration, helper activity, temporary paths, and underlying errors |

`--quiet`, `--verbose`, and `--debug` are mutually exclusive.

## stdout, stderr, and terminals

- `--help` writes to stdout.
- Human-readable progress, warnings, errors, and summaries write to stderr.
- `--format json` and `--format jsonl` machine data write to stdout.
- Interactive stderr terminals use one in-place progress region.
- Pipes, redirection, and CI automatically use plain line output without ANSI control sequences.

Default batch output does not print every successful file. Warnings and failures temporarily clear the dynamic progress region, print the message, and then restore progress.

## JSON and JSONL

`--format json` emits one JSON document containing an `events` array. `--format jsonl` emits one independent JSON object per line.

Every machine record contains `schema_version: 1`. Field names, event names, phase names, warning codes, and error codes are stable English identifiers. Only `message` may be localized.

```json
{"schema_version":1,"event":"conversion_failed","error_code":"source_gain_map_missing","input":"IMG_001.heic","message":"The source photo does not contain a usable HDR Gain Map."}
```

Current event names are:

- `conversion_started`
- `conversion_progress`
- `conversion_warning`
- `conversion_completed`
- `conversion_skipped`
- `conversion_failed`
- `batch_started`
- `batch_progress`
- `batch_completed`

## Stable error codes

| Error code | Meaning |
| --- | --- |
| `source_not_found` | The input path does not exist |
| `source_not_supported` | The source photo is unsupported |
| `source_gain_map_missing` | No usable HDR Gain Map is present |
| `source_gain_map_corrupt` | The Gain Map is incomplete or damaged |
| `portrait_data_unavailable` | Required portrait resources are unavailable |
| `apple_runtime_unavailable` | Required Apple processing is unavailable on this system |
| `output_not_writable` | The output cannot be created or replaced |
| `output_verification_failed` | The written output failed validation |
| `internal_container_error` | The container has an unsupported internal condition |
| `invalid_arguments` | The command or arguments are invalid |
| `batch_incomplete` | A batch completed with failures |

Default text shows only a user-readable reason and recovery suggestion. `--verbose` adds the error code; `--debug` adds container diagnostics and the complete underlying error chain.

## Language selection

Language resolution order:

1. `--language`
2. `XDREMUX_LANGUAGE`
3. System preferred languages
4. English fallback

Simplified Chinese identifiers are `zh-Hans` and `zh-CN`. English identifiers are `en`, `en-US`, and `en-GB`. Other languages currently fall back to English.

JSON fields, event names, error codes, option names, environment variables, filenames, and exit codes are never localized.

## Batch reruns and failure reports

Batch output preserves each input path relative to `--input-dir`, so equal filenames in separate albums remain distinct.

On rerun:

1. Existing outputs that pass lightweight validation are skipped.
2. Invalid or incomplete outputs are regenerated.
3. `--overwrite` always regenerates.
4. Each file is written to a sibling temporary file and atomically installed after validation.
5. One file failure does not stop remaining work.

Failures are written to `<output-dir>/xdremux-failures.json`. A later clean run removes an obsolete report. Batch recovery does not use checkpoint journals, configuration hashes, or mtime state machines.

## Exit codes

| Exit code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Internal container error |
| `2` | Invalid command or arguments |
| `3` | Missing, unsupported, or invalid input |
| `4` | Output or Apple runtime failure |
| `5` | Batch completed with one or more failures |
| `130` | Interrupted with Ctrl+C |

## Python CLI

The Python CLI retains the original HDR conversion path. It does not provide Apple Photographic Styles or Apple Portrait.

```bash
pip install pillow-heif Pillow numpy
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic
python3 xdremux/python/XDRemux.py batch --input-dir photo_dump/
python3 xdremux/python/XDRemux.py convert --oppo-compatible --input IMG_001.heic
```

The Swift CLI is the preferred entry point for new features and automation.

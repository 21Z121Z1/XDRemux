# CLI Output Policy

English | [简体中文](logging.md)

This document defines the current command-output contract.

For command options, see the [CLI reference](../cli.en.md).

## Human-readable output

The Rust CLI prints human-readable progress for normal product commands.

Use stdout for normal progress and result lines.

Use stderr for errors.

Do not add a second general logging protocol unless the CLI contract explicitly adds it.

## Machine-readable commands

The Rust commands below write JSON to stdout:

- `inspect --json`
- `batch --json`
- `categorize --json`
- `validate --json`

Do not mix unrelated progress text into the JSON stdout stream of these commands.

## Error text

An error message must first identify the user-visible problem.

Add an internal metadata key or container term only when it helps the reader diagnose the problem.

Keep one batch failure on one line when the batch UI depends on line-oriented output.

Preserve the original error when a higher layer adds context.

Do not replace a critical thrown error with an unstructured `print` and then continue as if the operation succeeded.

## Exit status

The Rust CLI uses `0` for success/help/version, `1` for runtime conversion or validation failures, and `2` for command-line syntax or usage errors. Do not invent a second product exit-code contract in the adapter or research tooling.

## Diagnostics

Some conversion paths print diagnostic lines that identify selected source facts or product outcomes.

Tests can depend on exact diagnostic text. Search for assertions before changing a diagnostic string.

`--debug-dir` can retain diagnostic artifacts for supported conversion paths.

Apple capability failures can also retain evidence or helper output when the platform operation requires it.

## Library boundary

The Rust runtime and crates are libraries. They should expose structured results, warnings, and errors instead of depending on terminal formatting.

Terminal formatting and localization belong at the CLI or app presentation layer.

Current technical writing and user-visible error guidance follow the [technical writing guide](../style-guide.en.md).

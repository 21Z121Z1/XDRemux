# CLI Output Policy

English | [简体中文](logging.md)

This document defines the current command-output contract.

For command options, see the [CLI reference](../cli.en.md).

## Human-readable output

Normal Swift and Python conversion commands print human-readable progress.

Use stdout for normal progress and result lines.

Use stderr for errors.

Do not add a second general logging protocol unless the CLI contract explicitly adds it.

## Machine-readable commands

The Swift commands below write JSON to stdout:

- `validate-apple`
- `validate-portrait`
- `portrait-self-test`

Do not mix unrelated progress text into the JSON stdout stream of these commands.

## Error text

An error message must first identify the user-visible problem.

Add an internal metadata key or container term only when it helps the reader diagnose the problem.

Keep one batch failure on one line when the batch UI depends on line-oriented output.

Preserve the original error when a higher layer adds context.

Do not replace a critical thrown error with an unstructured `print` and then continue as if the operation succeeded.

## Exit status

Swift and Python do not use the same parser implementation.

For Swift:

- normal success uses `0`;
- runtime failures use `1`;
- Swift Argument Parser owns parser-level usage exits;
- Motion Photo pre-routing failures caught by `main.swift` use `1`.

For Python:

- normal success uses `0`;
- runtime command failures use `1`;
- `argparse` parser-level usage errors use `2`.

Do not publish one shared numeric exit table unless both implementations actually share that behavior.

## Diagnostics

Some conversion paths print diagnostic lines that identify the selected implementation path.

Tests can depend on exact diagnostic text. Search for assertions before changing a diagnostic string.

`--debug-dir` can retain diagnostic artifacts for supported conversion paths.

Apple feature failures can also retain evidence or helper output when their implementation requires it.

## Library boundary

`XDRemuxCore` is a library. It should expose structured results, warnings, and errors instead of depending on terminal formatting.

Terminal formatting and localization belong at the CLI or app presentation layer.

Current technical writing and user-visible error guidance follow the [technical writing guide](../style-guide.en.md).

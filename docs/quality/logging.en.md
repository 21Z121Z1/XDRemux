# CLI output policy

English | [简体中文](logging.md)

The Rust CLI owns terminal output and exit status. This policy applies to its human-readable text, JSON results, and diagnostics. Command options are in the [CLI reference](../cli.en.md).

## Keep output streams separate

In human-readable mode, normal product commands write progress and results to stdout. They write errors to stderr. Do not add another general logging protocol unless the CLI contract requires one.

These command forms write JSON to stdout:

- `inspect --json`
- `batch --json`
- `categorize --json`
- `validate --json`

When a command reports failures in JSON, keep them in the structured stdout result. Parser and diagnostic errors can still use stderr. Keep unrelated progress text out of the JSON stream. Diagnostics and structured results serve different consumers.

## Write actionable errors

State the user-visible problem first. Include an internal metadata key or container term only when it helps diagnose that problem.

If the batch UI consumes one line per failure, preserve that format. When a higher layer adds context, retain the original error. Do not replace an error with an unstructured `print` and continue as though the operation succeeded.

## Preserve exit status

| Status | Meaning |
| --- | --- |
| `0` | Success, help, or version output. |
| `1` | Runtime conversion, validation, or output failure. |
| `2` | Syntax, usage, input discovery, or planning error. |

The adapter and research tools must not define a second product exit-code contract.

## Retain useful diagnostics

Some conversion paths report source facts or product outcomes as diagnostic lines. Before changing a diagnostic string, search for tests that assert its exact text.

The public CLI does not expose `--debug-dir`. Use the documented validation or diagnostic script for the affected feature. An Apple capability failure can retain evidence or helper output when the platform operation requires it.

## Keep formatting out of libraries

The runtime and other Rust crates expose structured results, warnings, and errors. They should not depend on terminal formatting. The CLI and app presentation layers own formatting and localization.

Follow the [technical writing guide](../style-guide.en.md) for documentation and user-visible errors.

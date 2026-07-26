# Output policy

English | [简体中文](logging.md)

What XDRemux writes to the terminal, and where. For command output from a user's point of view, see the [CLI reference](../cli.en.md).

## How it works today

The CLI emits human-readable text only. There is no JSON event stream, and no `--quiet`, `--verbose`, or `--format` switch.

- Progress and results go to **stdout**: `converted X -> Y`, `skipped X (output already up to date)`, `batch complete: N converted, N skipped, N failed -> <dir>`.
- Errors and diagnostic notes go to **stderr**, all prefixed with `error:`.
- Exit codes are only `0` (success) and `1` (any error).

`validate-apple`, `validate-portrait`, and `portrait-self-test` are the exception: they write JSON to stdout, so it can be redirected straight to a file.

## Error text

`XDRemuxError` has two renderings:

- `description` is the full form. It may span lines and is used by single-file `convert`: the first line says what happened, the rest explains why and what to do next.
- `headline` is the one-line form used in batch listings, so one file stays one line.

When writing a new message: **lead with something the reader can understand, then give the technical detail.** An internal block name is not a first sentence — `not a ProXDR photo` is for a person, `local.hdr.meta.data` is not.

Error text in `XDRemuxCore` stays English because the module is a public Swift package. Chinese localization belongs at the presentation layer, in the macOS app's `AppStrings.failureReason`.

## Conversion diagnostics

The converter prints a small number of diagnostic lines that identify which path a run took, for example:

```text
[direct-gain] preserved compressed Base; encoded 15 Gain Map tiles once quality=0.90 tile=512x512
```

These lines **are asserted by tests** — `verify_swift_cli_sample.py --expect-direct-gain` counts occurrences of one — so check for an assertion before rewording them.

## Investigating a failure

- `--debug-dir <dir>` keeps the run's intermediate artifacts.
- Apple Photographic Styles **keeps its evidence directory on failure** and prints the path to stderr; it is cleaned up only on success.
- Finer debugging switches are in the environment-variable table in the [development guide](../development.en.md).

## Rules for code

1. Do not swallow an exception on a critical path with a bare `print`.
2. When catching, keep the original error rather than flattening it to one sentence.
3. In batch paths, one file gets one line; multi-line explanations belong to the single-file path.

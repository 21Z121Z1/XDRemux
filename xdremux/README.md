# XDRemux Converter Entrypoints

This directory contains compatibility and non-SwiftPM converter entry points.

- The recommended macOS Swift CLI is the root package product:
  `swift run xdremux ...`.
- `swift-cli/` preserves the legacy `swift <file>` entry point and forwards to
  the root package executable.
- `python/` is the cross-platform Python CLI path.

Both current CLIs support standalone shooting-mode categorization with
`categorize --input ... --output-dir ...`. Their `batch` commands use the
`--categorize` switch to write converted files under the same Chinese mode
directories. Single-file `convert` does not accept that switch.

Reusable Swift implementation lives under `Sources/`. Graphical app shells are
intentionally kept outside this directory under `apps/`.

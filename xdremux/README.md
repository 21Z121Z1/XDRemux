# XDRemux Converter Entrypoints

This directory contains compatibility and non-SwiftPM converter entry points.

- The recommended macOS Swift CLI is the root package product:
  `swift run xdremux ...`.
- Internal conversion controls and validators use `swift run xdremux-dev ...`.
- `swift-cli/` preserves the legacy `swift <file>` entry point and forwards to
  the root package executable.
- `python/` is the cross-platform Python CLI path.

Reusable Swift implementation lives under `Sources/`. Graphical app shells are
intentionally kept outside this directory under `apps/`.

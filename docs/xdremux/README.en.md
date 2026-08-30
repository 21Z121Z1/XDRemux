# Technical Implementation Index

English | [简体中文](README.md)

This directory indexes stable implementation contracts for XDRemux.

Use the [project README](../../README.en.md) for normal use and the [CLI reference](../cli.en.md) for command behavior.

## Current architecture

### `XDRemuxCore`

`XDRemuxCore` owns format and conversion logic that does not require the Apple feature layer.

Current responsibilities include:

- ProXDR metadata parsing;
- ISO/TS 21496-1 Gain Map conversion;
- HEIF and ISO-BMFF parsing and writing;
- Motion Photo parsing and resource extraction;
- source metadata and classification;
- output validation shared by core conversion paths.

### `XDRemuxAppleFeatures`

`XDRemuxAppleFeatures` owns Apple-specific conversion and validation.

Current responsibilities include:

- Motion Photo to Apple Live Photo;
- Live Photo still and MOV writing;
- Live Photo timing and asset identity;
- vendor-specific geometry policy used by the Live Photo writer;
- Photographic Styles;
- Apple Portrait;
- Apple-specific native helper integration.

### CLI layer

`Sources/XDRemuxCLI/` owns user command parsing and routing.

The CLI automatically routes supported Motion Photo inputs before the normal HDR command path.

The Motion Photo and normal HDR paths have different output-safety rules. See the [CLI reference](../cli.en.md).

### Python implementation

`xdremux_py/` is a separate cross-platform implementation.

It supports standard HDR conversion, Motion Photo to Live Photo conversion, and classification. It does not implement Photographic Styles or Apple Portrait generation.

## Stable media contracts

### Standard HDR

The standard path converts vendor HDR metadata to an ISO/TS 21496-1 representation.

The implementation tries to preserve source compressed image data when the selected path permits it.

### Live Photo

The normal Live Photo path publishes an HEIC/HEIF still and a MOV with a shared asset identifier.

The converter maps the resolved source cover time to Apple `still-image-time`.

The normal MOV writer uses compressed video/audio passthrough. The validator compares source and output compressed samples where required by the path.

The still writer preserves the source Gain Map when one is present and the output path supports it.

### Publication

Live Photo output is a pair transaction. A conversion must not leave one final member as a successful result while the other member fails publication.

Batch reuse requires source provenance. A valid pair with unknown lineage is not accepted as the output of an unrelated input.

## Current technical documents

- [Apple feature guide](../apple-features.en.md)
- [Development guide](../development.en.md)
- [Testing policy](../quality/testing.en.md)
- [Validation runbook](../validation/README.en.md)
- [Fixture guide](../../fixtures/README.en.md)

## Historical audit

[ISO conformance audit, 2026-05-11](iso-conformance-audit-20260511.md) is a historical record.

It contains paths and implementation details from that date. Do not use its old paths as the current architecture reference.

Preserve historical audit measurements. Add a new dated audit when a new conformance study supersedes them.

Current technical documentation follows the [technical writing guide](../style-guide.en.md).

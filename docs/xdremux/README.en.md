# v1.4 Technical Implementation Index

English | [简体中文](README.md)

This directory indexes stable implementation contracts for the released XDRemux v1.4 Swift/Python line.

For system-wide ownership, architectural layers, and branch roles, use the [system architecture](../architecture.en.md). For the Rust transition, use the [transition roadmap](../roadmap.en.md). Do not use the v1.4 directory structure below as the future architecture specification.

Use the [project README](../../README.en.md) for normal use and the [CLI reference](../cli.en.md) for command behavior.

## v1.4 implementation structure

### `XDRemuxCore`

`XDRemuxCore` owns format and conversion logic that does not require the Apple feature layer in v1.4.

Current v1.4 responsibilities include:

- ProXDR metadata parsing;
- ISO/TS 21496-1 Gain Map conversion;
- HEIF and ISO-BMFF parsing and writing;
- Motion Photo parsing and resource extraction;
- source metadata and classification;
- output validation shared by core conversion paths.

### `XDRemuxAppleFeatures`

`XDRemuxAppleFeatures` owns Apple-specific conversion and validation in v1.4.

Current v1.4 responsibilities include:

- Motion Photo to Apple Live Photo;
- Live Photo still and MOV writing;
- Live Photo timing and asset identity;
- vendor-specific geometry policy used by the Live Photo writer;
- Photographic Styles;
- Apple Portrait;
- Apple-specific native helper integration.

### CLI layer

`Sources/XDRemuxCLI/` owns v1.4 user command parsing and routing.

The CLI automatically routes supported Motion Photo inputs before the normal HDR command path.

The Motion Photo and normal HDR paths have different output-safety rules. See the [CLI reference](../cli.en.md).

### Python implementation

`xdremux_py/` is the separate cross-platform v1.4 implementation.

It supports standard HDR conversion, Motion Photo to Live Photo conversion, and classification. It does not implement Photographic Styles or Apple Portrait generation.

During the Rust transition, use these implementations as bounded behavioral references where current contracts or independent evidence support them. Do not preserve their file/module split merely for symmetry.

## Stable media contracts

These contracts remain migration inputs even when their implementation owner changes.

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

Publication, provenance, collision handling, and crash recovery are product correctness contracts. The Rust rewrite must preserve them through explicit Layer 5 ownership rather than leaving them as CLI-specific behavior.

## Current technical documents

- [System architecture](../architecture.en.md)
- [Transition roadmap](../roadmap.en.md)
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

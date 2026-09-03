# Technical Implementation Index

English | [简体中文](README.md)

This directory indexes stable implementation contracts for XDRemux. The Rust workspace is the only product implementation; Swift targets remain only at explicitly marked migration-oracle or Apple-primitive boundaries.

Use the [project README](../../README.en.md) for normal use and the [CLI reference](../cli.en.md) for command behavior.

## Current architecture

### Rust workspace

The Rust `xdremux` CLI and workspace crates own all product semantics, format conversion, classification, batching, Motion Photo routing, validation, and publication.

Current responsibilities include:

- ProXDR metadata parsing;
- ISO/TS 21496-1 Gain Map conversion;
- HEIF and ISO-BMFF parsing and writing;
- Motion Photo parsing and resource extraction;
- source metadata and classification;
- output validation shared by core conversion paths;
- cross-platform policy, manifest construction, and transaction orchestration for Apple Portrait and Photographic Styles.

### Apple capability boundary

`Sources/XDRemuxAppleAdapter/` is the minimal Apple-framework primitive adapter invoked by the Rust runtime.

Current responsibilities include:

- invoking ImageIO, Vision, CoreImage, VideoToolbox, and other Apple frameworks;
- returning framework facts through the Rust-defined protocol;
- writing or probing Apple-specific resources already planned by Rust.

`Sources/XDRemuxCore/` and `Sources/XDRemuxAppleFeatures/` remain only as migration oracles pending deletion. They are not product entry points and must not receive new product policy.

### CLI layer

`crates/xdremux-cli/` owns the only user command parser and router.

The CLI automatically routes supported Motion Photo inputs before the normal HDR command path.

The Motion Photo and normal HDR paths have different output-safety rules. See the [CLI reference](../cli.en.md).

### Python implementation

`xdremux_py/` may remain only as migration-oracle, fixture, and research/training tooling.

It does not participate in the Rust runtime, define a formal conversion path, or serve as the canonical CI correctness source.

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

# Technical Implementation Index

English | [简体中文](architecture.md)

This document indexes stable implementation contracts for XDRemux. The Rust workspace is the only product implementation; Swift contains only the explicitly bounded Apple primitive adapter.

Use the [project README](../README.en.md) for normal use and the [CLI reference](cli.en.md) for command behavior.

## Current architecture

### Rust workspace

The Rust `xdremux` CLI and workspace crates own all product semantics, format conversion, classification, batching, Motion Photo routing, validation, and publication.

Current responsibilities include:

- ProXDR metadata parsing;
- ISO/TS 21496-1 Gain Map conversion;
- HEIF and ISO-BMFF parsing and writing;
- Motion Photo routing and resource extraction through the independent LibLivePhoto facade;
- source metadata and classification;
- output validation shared by core conversion paths;
- cross-platform policy, manifest construction, and transaction orchestration for Apple Portrait and Photographic Styles.

LibLivePhoto supplies the portable Motion/Live Photo format implementation.
XDRemux retains HDR processing, codec choices, inferred-time policy, geometry and
filesystem publication. See [the integration contract](liblivephoto-integration.en.md).

### Apple capability boundary

The complete Apple product ships only for arm64 on macOS 27 or newer. Its App and native package share that build target, and the adapter rejects unsupported hosts before capability advertisement. Cross-platform Rust core tests remain independent; native framework and device acceptance still require their actual environments.

`Sources/XDRemuxAppleAdapter/` is the minimal Apple-framework primitive adapter invoked by the Rust runtime.

Current responsibilities include:

- invoking ImageIO, Vision, CoreImage, VideoToolbox, and other Apple frameworks;
- returning framework facts through the Rust-defined protocol;
- writing or probing Apple-specific resources already planned by Rust.

The former Swift product targets have been removed. Only `Sources/XDRemuxAppleAdapter/` remains on the Swift side, and it must not receive product policy.

### CLI layer

`crates/xdremux-cli/` owns the only user command parser and router.

The CLI automatically routes supported Motion Photo inputs before the normal HDR command path.

The Motion Photo and normal HDR paths have different output-safety rules. See the [CLI reference](cli.en.md).

### Python research tooling

`xdremux_py/` contains only research/training tooling.

It does not participate in the Rust runtime, define a formal conversion path, or serve as the canonical CI correctness source.

### Model primitives

`model_adapters/geocalib/` is a separate local inference component. The Rust runtime reads recorded equivalent-focal metadata through ExifTool and invokes the model only when that metadata is absent. The Python helper returns source-bound pixel intrinsics and uncertainty. Rust owns source precedence, fixed-camera admission, equivalent-focal conversion, native generation and publication. The helper is not a second converter or a user CLI. Its pinned environment and weights require separate installation; no media is uploaded.

## Stable media contracts

### Standard HDR

The standard path converts vendor HDR metadata to an ISO/TS 21496-1 representation.

The implementation tries to preserve source compressed image data when the selected path permits it.

### Live Photo

The default Live Photo path publishes an HEIC/HEIF still and a MOV with a shared asset identifier. The typed `LivePhotoOutput` intent optionally wraps that same validated pair in a real PVT directory; it does not cross a codec/remux boundary again. Rust owns its bounded package layout, validation and publication. The App forwards the choice and displays the planned container.

The converter maps the resolved source cover time to Apple `still-image-time`.

The normal MOV writer uses compressed video/audio passthrough. The validator compares source and output compressed samples where required by the path.

The still writer preserves the source Gain Map when one is present and the output path supports it.

### Publication

With the default pair output, Live Photo conversion succeeds only after both final members are published. It writes a validated, synced pair into its own private staging directory, then claims the MOV name before the still name using atomic no-clobber hard links. Existing final entries and unrelated files or directories are never replaced or deleted. Cooperating writers share the MOV claim, including `.heic` and `.heif` outputs with the same stem.

Two filesystem names are not a system-atomic transaction. If the second link fails, conversion returns an error identifying the partial MOV and retains the complete staged pair for explicit recovery. The staged `capture.heic` can be checked with `xdremux validate`; inspect both final entries before moving either member. Reconciliation only checks destinations: unknown partial or invalid pairs are preserved and rejected. Stale-looking names do not establish ownership.

Publication requires same-filesystem hard-link support. Unsupported filesystems fail closed with the original I/O error; FAT/exFAT support is not claimed. This protocol cannot prevent an external writer from actively replacing final entries. Synced staging files do not establish directory-entry durability after power loss.

PVT publication writes the validated media and fixed XML version plist into an
operation-owned private directory. A platform exclusive directory rename claims
the single final package path without replacing even an empty existing directory.
The deletion guard is disarmed before moving the directory, so it never cleans
a later entry at the old staging pathname. A publication failure retains the
complete package and reports its recovery path. macOS/Linux backends fail closed
when exclusive rename is unavailable; other core platforms can read/validate the
supported layout but do not silently use a replacing publisher. No directory
power-loss durability or Photos/wallpaper acceptance is inferred.

Batch reuse requires source provenance and matching container identity. A valid
pair or package with unknown lineage is not accepted as the output of an
unrelated input. The App has no durable source checkpoint for packages and
therefore does not automatically adopt an existing PVT.

## Current technical documents

- [Apple feature guide](apple-features.en.md)
- [Development guide](development.en.md)
- [Testing policy](quality/testing.en.md)
- [Validation runbook](validation/README.en.md)
- [Fixture guide](../fixtures/README.en.md)

## Historical audit

[ISO conformance audit, 2026-05-11](history/iso-conformance-audit-20260511.md) is a historical record.

It contains paths and implementation details from that date. Do not use its old paths as the current architecture reference.

Preserve historical audit measurements. Add a new dated audit when a new conformance study supersedes them.

Current technical documentation follows the [technical writing guide](style-guide.en.md).

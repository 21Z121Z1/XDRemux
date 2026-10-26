# Product architecture and media contracts

English | [简体中文](architecture.md)

The Rust workspace owns XDRemux's product behavior. The Apple adapter performs framework operations that Rust requests. Python contains research and training tools.

This document explains ownership and output safety. For commands, use the [CLI reference](cli.en.md). For a first conversion, use the [project README](../README.en.md).

## Component ownership

| Component | Responsibility |
| --- | --- |
| `crates/xdremux-cli/` | Parse public commands and route the requested operation. |
| `crates/xdremux-engine/` | Model source facts, user intent, capabilities, and conversion policy. |
| `crates/xdremux-runtime/` | Execute the plan, validate results, and publish output files. |
| Rust media crates | Parse metadata, process HDR, encode images, and read or write containers. |
| LibLivePhoto | Provide the portable Motion Photo and Live Photo format implementation. |
| `Sources/XDRemuxAppleAdapter/` | Call Apple frameworks and return observations through the Rust-defined protocol. |
| `Sources/XDTextureNative/` and `Sources/XDCinematicNative/` | Implement native Texture and Cinematic operations for the adapter. |
| `apps/macos/XDRemuxApp/` | Present the Mac interface and invoke the product operations. |
| `xdremux_py/` | Provide research and training tools outside the product runtime. |

### Rust product stack

Rust owns ProXDR metadata parsing, ISO/TS 21496-1 Gain Map conversion, HEIF assembly, classification, batching, validation, and publication. It also owns the resource models and transaction policy for Apple Portrait and Photographic Styles.

LibLivePhoto supplies source inspection, resource selection, exact presentation time, and pair validation. XDRemux retains HDR processing, codec selection, inferred-time policy, geometry, and filesystem publication. See the [LibLivePhoto integration contract](liblivephoto-integration.en.md).

The CLI detects supported Motion Photo inputs before it starts the ordinary HDR path. These paths have different output-safety rules. The [CLI reference](cli.en.md) defines those rules.

### Apple framework operations

The complete Apple product requires Apple silicon (`arm64`) and macOS 27 or newer. The App and native package use the same deployment target. The adapter rejects an unsupported host before it reports capabilities.

The adapter calls ImageIO, Vision, Core Image, VideoToolbox, and other required Apple frameworks. It returns framework observations or writes resources that Rust has requested. It does not choose product compatibility policy.

The Swift package publishes `xdremux-apple-adapter`. Its native Texture and Cinematic targets support that executable. The package does not provide a second public conversion CLI. See [Package.swift](../Package.swift) and the [Apple feature guide](apple-features.en.md).

Portable Rust tests remain available on Linux, Windows, and macOS. Those tests do not establish native framework availability or device behavior.

### Research tools

`xdremux_py/` does not participate in the Rust runtime or define a public conversion path. Research results can inform product changes. They do not replace product tests or native acceptance checks.

### Model primitives

`model_adapters/geocalib/` is a separate local inference component. The Rust runtime reads recorded equivalent-focal metadata through ExifTool and invokes the model only when that metadata is absent. The Python helper returns source-bound pixel intrinsics and uncertainty. Rust owns source precedence, fixed-camera admission, equivalent-focal conversion, native generation and publication. The helper is not a second converter or a user CLI. Its pinned environment and weights require separate installation; no media is uploaded.


## Media contracts

### Standard HDR

The standard path converts vendor HDR metadata to an ISO/TS 21496-1 representation. It preserves compressed source image data where the selected conversion path permits this. Conversion does not imply that every source resource can remain unchanged.

### Live Photo

The default output contains an HEIC/HEIF still and a MOV with the same asset identifier. The converter maps the resolved source cover time to Apple `still-image-time`.

The MOV writer copies compressed video and audio samples without decoding and re-encoding them. The runtime compares source and output media payloads before publication. The still writer preserves a source Gain Map when the output path supports it.

The `LivePhotoOutput` intent can place the same validated pair in a real PVT directory. Packaging does not encode or remux the media again. Rust owns the package layout, validation, and publication. The App passes the selected output type and displays the planned container.

### Publication

For a Live Photo pair, conversion succeeds only after both final files have been published and their bytes have passed readback. The runtime first writes and synchronizes a validated pair in a private staging directory. It then claims the MOV name with an atomic hard link that cannot replace an existing entry. It publishes the still file next.

Existing output entries and unrelated files or directories remain unchanged. Cooperating writers compete for the same MOV name. This also applies to `.heic` and `.heif` output names with the same stem.

Two filesystem names do not form one atomic transaction. If the second link fails, the command reports the partial MOV path and retains staging. A final-byte readback failure also retains staging and records the observed final entries.

The runtime attempts to write independent `recovery.heic` and `recovery.mov` files from the checked buffers. It synchronizes these files and compares their bytes with those buffers. The error confirms a complete independent recovery pair only if these operations succeed. Otherwise, it reports `recovery-unverified`. The original `capture.*` files can share damaged final inodes.

If the error confirms an independent checked recovery pair, validate the reported `recovery.heic` with `xdremux validate`. Before moving either member, inspect both final paths. Reconciliation checks existing destinations but preserves and rejects unknown partial or invalid pairs. A temporary-looking name does not prove that the operation owns that entry.

Pair publication requires hard links on the same filesystem. If the filesystem does not support them, publication fails with the original I/O error. FAT/exFAT support is not claimed. The protocol cannot stop an external writer from replacing files after readback. Synchronizing staging files does not guarantee directory-entry durability after power loss.

PVT publication uses a different filesystem operation. The runtime writes the validated media and fixed XML version plist in an operation-owned private directory. An exclusive directory rename claims the final package path. It cannot replace an existing directory, including an empty directory.

Before that rename, the runtime disables automatic deletion of the old staging pathname. It therefore cannot delete a new entry that later appears at that old path. If publication fails, the complete package remains available at the reported recovery path.

macOS and Linux publication fail when exclusive rename is unavailable. Other core platforms can read and validate the supported layout. They do not fall back to a rename that replaces an existing package. PVT structure does not prove power-loss durability, Photos import, or wallpaper support.

Batch reuse requires matching source provenance and container identity. A valid pair or package with unknown provenance cannot stand in for the output of another input. The App has no persistent source checkpoint for packages, so it does not automatically adopt an existing PVT.

The implementation is in [pair_publication.rs](../crates/xdremux-runtime/src/live_photo/pair_publication.rs) and [live_photo_package.rs](../crates/xdremux-runtime/src/live_photo_package.rs).

## Further reading

- [Apple feature guide](apple-features.en.md) defines platform requirements and evidence limits.
- [Development guide](development.en.md) describes builds and component ownership.
- [Testing policy](quality/testing.en.md) defines the evidence required for a change.
- [Validation runbook](validation/README.en.md) describes checks tied to a committed revision.
- [Fixture guide](../fixtures/README.en.md) defines source-media identity and provenance.

## Historical audit

The [ISO conformance audit of 2026-05-11](history/iso-conformance-audit-20260511.md) records the implementation at that time. Its old paths are not current architecture references.

Preserve the audit's measurements. If a new conformance study supersedes a conclusion, add a new dated record. Current documents follow the [technical writing guide](style-guide.en.md).

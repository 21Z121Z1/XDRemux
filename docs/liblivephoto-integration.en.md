# LibLivePhoto integration

English | [简体中文](liblivephoto-integration.md)

LibLivePhoto provides the portable Motion Photo and Live Photo format implementation. XDRemux uses its resource interface for inspection, classification, primary-resource selection, exact presentation time, and pair validation.

The two Git dependencies use the same full revision in their Cargo manifests and `Cargo.lock`. LibLivePhoto builds outside the XDRemux workspace. The dependency declarations are in [xdremux-format](../crates/xdremux-format/Cargo.toml) and [xdremux-motion-photo](../crates/xdremux-motion-photo/Cargo.toml).

## Ownership

| Owner | Contract |
| --- | --- |
| `liblivephoto-format` | Cursor, error, FourCC, JPEG, raw EXIF, and BMFF types re-exported by `xdremux-format`. |
| LibLivePhoto | Source resources, file-format parsing, pairing identity, and exact presentation time. |
| `xdremux-motion-photo` | Product geometry, file copying, and pair publication that preserves existing entries. |
| XDRemux codec and runtime | HDR policy, codec profiles, image encoding, conversion reports, and validated publication. |

New format operations use `MotionPhoto`, `Input`, resources, and `MediaTime`. The consumer crate must not introduce an independent format parser.

The integration retains the product's HDR application-color-space flag, Photographic Styles, and Cinematic routes. Shared format parsing belongs to LibLivePhoto. HDR decisions and codec operations belong to XDRemux.

## Copy an exact payload range

The local `copy_payload_range` and `copy_payload_range_with_options` functions stage the requested bytes beside the destination. They replace the destination only after the copy completes and the staging file is synchronized. A failed copy preserves the input and any existing destination.

A destination entry that participates in input-path resolution is rejected. This includes aliases of that entry. A distinct hard link or safe destination symlink can be replaced as an entry without changing its target.

Alias checks can require directory read permission. If a check fails, both entries remain intact. Callers must keep path resolution stable during the copy. This contract does not guarantee durability after power loss.

The implementation and its regressions are in [payload.rs](../crates/xdremux-motion-photo/src/payload.rs).

## Publish a pair

### Public helper

`publish_live_photo_pair` preserves caller-owned staging files on success and failure. Staging can reside in a child directory on the same filesystem. The helper never replaces or unlinks final entries.

If publication of the second member fails, `PairPublicationError::partial_output()` returns the observed partial-output path. Another writer can change that entry later. The reported path does not authorize deletion.

This helper has no source-media or expected-identity input. It therefore cannot perform the runtime's complete product validation. `reconcile_live_photo_pair` preserves unknown partial pairs, invalid pairs, and all sidecar files. See [publication.rs](../crates/xdremux-motion-photo/src/publication.rs).

### Runtime publisher

The private runtime publisher accepts owned media bytes only after all required checks pass:

- Strict pair parsing and validation.
- The expected asset identifier.
- The exact presentation time.
- Equal compressed media payloads in the source and output movies.

The publisher owns staging and file synchronization. Bounded reads compare staged and final files with immutable media buffers. If second-member publication or final-byte readback fails, the publisher retains staging and records the final entries it observed.

It attempts to write independent `recovery.heic` and `recovery.mov` files from the checked buffers. It synchronizes and reads back both files. The error confirms a complete independent recovery pair only if those operations succeed. Otherwise, it reports `recovery-unverified`. The original `capture.*` files can share damaged final inodes. These observations do not authorize deletion.

Callers must keep path resolution stable. These checks cannot stop an active same-user writer from changing files after readback. The [publication contract](architecture.en.md#publication) defines filesystem requirements and the limits of a two-file transaction.

## Presentation time and geometry

The runtime validates the primary movie's video track and sample table even when the source already supplies a presentation time. It preserves that time as an integer with a timescale. It does not round a known XMP timestamp to a video frame.

When the source has no presentation time, product policy selects a fallback position. The conversion receipt marks that position as inferred.

Apple geometry metadata accepts explicit values from the caller. OPPO matrix and stabilization policy remain in XDRemux. They are not defaults of the format library.

The library reader derives the supported primary movie MIME type from `ftyp`. Android JPEG/HEIF directory output retains QuickTime MIME without changing movie bytes. Codec conformance and Gallery acceptance require separate checks.

## HDR and resource preservation

JPEG and Gain Map encoding remain product operations. If the SEF fallback finds valid HDR data without a mapped Gain Map resource, conversion fails. It does not silently create an SDR output.

The Live Photo receipt records exact presentation time, compressed movie comparisons, metadata changes, required re-encoding, generated identity, and omitted resources. Both `convert` and newly converted `batch` items report omissions. A resumed item has an unknown omission count because its checkpoint does not contain a conversion report.

The original source remains intact. Product conversion to an Apple pair does not promise preservation of every vendor resource or a lossless result. The library's source-copy mode or explicit sidecar policy is the relevant contract for that requirement.

## Optional PVT container

PVT is a runtime container for an already validated Apple pair. The runtime writes a real directory with unchanged media and a bounded XML plist. It publishes the directory exclusively.

PVT does not introduce another MOV/HEIF parser, asset identifier, codec operation, or remux. LibLivePhoto still owns media parsing, pairing identity, and exact presentation time. Pair output remains the default.

See [CLI packaging](cli.en.md#optional-pvt-packaging) and [publication](architecture.en.md#publication) for collision and recovery rules. A valid package structure does not establish Photos import or dynamic wallpaper support.

## Verification and evidence limits

The existing parser and payload conformance checks cover the consumer boundary. The real-fixture CLI gate converts and validates all 14 pinned Motion Photo inputs. Runtime tests compare the known source time with the written time and check the conversion report.

The public consumer contract uses an original ColorOS 16 dual-stream fixture. The default Apple plan rejects unmapped vendor resources. Explicit sidecar policy preserves each unmapped resource byte-for-byte. Re-reading the pair checks exact time and encoded-media equality. A truncated movie is rejected.

This test does not identify the semantic role of the second video stream or prove Gallery support. See [liblivephoto_interop.rs](../crates/xdremux-motion-photo/tests/liblivephoto_interop.rs) and the [real-fixture gate](../scripts/validation/check_rust_motion_photo_real_fixtures.sh).

The completion gate binds local checks to the committed `HEAD`. GitHub runs the consumer build and checks in a separate environment. Reader, writer, fixture, CI, and device results must remain separate claims.

The standalone library converter and external-workspace check exercise Android/Apple round trips without the XDRemux product stack. The library compatibility documentation specifies the integer-microsecond restriction for Apple-to-Android output. Original Gallery and Photos acceptance remain independent requirements. The pinned library records [format support](https://github.com/21Z121Z1/LibLivePhoto/blob/794ab552596db0f66095600d23cf7937a631b9ec/FORMAT_SUPPORT.md) and [vendor evidence](https://github.com/21Z121Z1/LibLivePhoto/blob/794ab552596db0f66095600d23cf7937a631b9ec/evidence/VENDORS.md).

## Compatibility feature

`xdremux-compat` provides unstable migration re-exports for existing low-level callers. It is not a stable public API.

Remove a re-export only after its caller uses a public resource or Apple API and the corresponding conformance check passes. Keep payload copying, safe publication, and product geometry local. Do not remove the entire compatibility feature before those callers have migrated.

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

ProXDR conversion preserves the primary HEVC data and its source ICC/nclx. The portable RGB Gain Map encoder uses BT.601 full-range YCbCr; the gain grid and tiles declare nclx `(2, 2, 6)`. The first two fields are unspecified as required by [HEIF tone-map derivation](https://www.iso.org/standard/89044.html). They do not describe the reconstructed HDR gamut. Standard RGB output uses 4:4:4; OPPO compatibility retains 4:2:0.

The default tmap colorimetry is BT.2020 PQ. This describes a derived image: the decoder applies the gain in the primaries selected by `use_base_colour_space`, then converts to the alternate encoding as specified by [ISO 21496-1](https://www.iso.org/standard/86775.html), Annex B. No stored PQ primary is implied. When ISO APP2 metadata is selected, conversion preserves an alternate ICC when supplied; Android hdrgm defines HDR primaries from the base profile and does not use a Gain JPEG ICC as alternate colorimetry. Without one, the gain application retains the base primaries even if the source flag is zero (C.4.7). Untagged JPEG RGB uses a bundled standard sRGB ICC so the encoded base has explicit colorimetry (C.4.4).

Styles reconstruction and HDR Look fit scene pixels in linear Display P3. When generating a new RGB Gain Map, Rust converts both SDR and HDR pixels to linear BT.2020, then computes new channel gains in that space. These outputs select the alternate primaries (`use_base_colour_space=0`) and keep BT.2020 PQ on tmap. A common offset covers negative linear components without clipping them before the logarithmic ratio. This preserves the gain operation's working space even when the unchanged SDR primary uses sRGB or another profile. It does not change the ProXDR gain-map application space.

For ISO JPEG input, Exif `ColorSpace=1` selects sRGB even when an ICC is present (C.4.4). Other Exif values retain the source ICC when available. Android hdrgm input retains its existing ICC selection.

### JPEG gain-map metadata selection

The runtime locates a complete gain JPEG after the baseline image and checks any declared GainMap length before parsing its metadata. The baseline's version-only ISO APP2 is not a gain-map payload. On the located gain JPEG, valid ISO APP2 takes precedence over Adobe hdrgm XMP, including conflicting values. This follows the [Android Ultra HDR v1.1 compatibility recommendation](https://developer.android.com/media/platform/hdr-image-format#iso_21496-1_compatibility). It is not an ISO-defined cross-format priority. Valid ISO is read without parsing XMP. Only absent ISO permits the XMP path.

Present but malformed or unsupported ISO fails conversion, even if XMP is valid or the Motion Photo has no mapped GainMap resource. The error identifies the ISO failure. It cannot become an XMP or SDR success. This is XDRemux's conservative conversion policy. ISO C.2.3 instead tells a general reader to ignore an unknown minimum version and display the baseline. A converter failure leaves the source available and preserves existing outputs.

ISO validation compares each signed gain interval and the unsigned baseline/alternate headrooms by exact 64-bit integer cross products before creating floating-point observations. Each minimum must be no greater than its maximum, gamma must be positive, and the headrooms must differ. The headrooms may decrease for an HDR baseline. Zero rational denominators, unsupported minimum versions, truncated required fields and nonzero reserved flags fail. Higher writer versions and optional trailing bytes remain accepted when the minimum version and required fields are supported. XMP keeps its Android color binding and defaults, with separate checks for nonnegative `HDRCapacityMin` and strictly larger `HDRCapacityMax`. The existing real JPEG corpus has no capacity conflict with those checks.

The required ISO rational pairs, metadata component count and writer version flow directly to the HEIF writer without decimal rounding, clamping, or an exp2/log2 round trip. Supported optional tails may be discarded. Public float observations cannot be edited independently of these source bytes. XMP and newly generated scene metadata use bounded continued fractions with signed numerators for gains/offsets and unsigned numerators for gamma/headrooms. Projection error must not exceed max(1e-12, abs(value) * 1e-9), and nonzero values cannot become zero. Out-of-range or unrepresentable values fail. The emitted tmap is parsed and its exact numeric constraints are checked again. Compatibility XMP uses round-trip float formatting; ISO headrooms use up to 40 decimal places and exact direction selection so distinct fractions cannot collapse to equal text. Without an alternate ICC, C.4.7 selects baseline application primaries even for a zero source flag. `tmap` references exactly `[base, gain]`; each image has its own color property. Gain compression remains full-range BT.601 nclx `(2, 2, 6)`. `tmap.colr` describes the alternate representation and can carry its supplied ICC. The [sample audit](validation/20261028-iso-jpeg.en.md) records the compatibility scope and commands.

An ISO alternate ICC must pass header, length, class, RGB/PCS, tag bounds/alignment/uniqueness, reserved-byte and overlap checks. Exact tag-data sharing is allowed. The profile must contain description, copyright, media white point, and matrix/TRC or device-to-PCS transform data. A bounded pure Rust CMM (`moxcms`, pinned by Cargo.lock) parses the profile and executes relative-colorimetric forward/reverse transforms in f64. CICP overriding is disabled. Non-finite or degenerate transforms fail. HDR alternates (alternate log2 headroom greater than zero) require CICP; RGB CICP uses matrix 0.

CICP consistency is checked on a 9-cubed RGB lattice plus 257-sample neutral and individual-channel ramps. Actual ICC transforms and the specified CICP encoding must agree within 0.002 + 0.005 * abs(reference) in normalized linear RGB per component. These are product admission tolerances, not ICC conformance thresholds or a final image error budget. Unspecified CICP fields retain matrix/TRC profile data; partially unspecified LUT bindings cannot be verified and fail explicitly. A limited-range CICP flag is legal, but its RGB normalization needs a known component bit depth; this alternate-ICC path currently rejects it as unsupported rather than guessing. HLG consistency uses the CMM's nominal system-gamma-1.2 reference; profiles using other black-level/OOTF conventions may be rejected as outside this policy.

A present invalid or unsupported profile fails conversion before publication and is never treated as missing or replaced with a guessed gamut. Accepted original bytes remain in `tmap.prof`; the admission CMM does not recolor product pixels. Baseline ICC admission and XMP-only gain ICC handling retain their existing behavior. Finite sampling and one CMM/intent do not establish equivalence for all profiles, rendering intents or native consumers. The [follow-up validation](validation/20261028-color-integrity.en.md) records the counterexamples and remaining gaps.

### Color precision and coverage

The managed linear RGBA32F working path and explicit P3-to-BT.2020 conversion preserve extended and negative components until gain construction. The final generated gain remains 8-bit and uses lossy HEVC quality 90. For gamma 1 and gain span S EV, rounding alone can introduce up to S / (2 * 255) EV, or a multiplicative gain error of 2^(S / (2 * 255)) - 1. At the allowed 24-EV span this is about 3.32%, before compression. This is an error in gain, not a bound on final scene RGB or perceptual color difference; offsets and black-level cancellation matter.

A codec regression measures quantization plus actual HEVC encode/decode on a fixed smooth RGB ramp for 4:4:4 and 4:2:0. It reports code-value, maximum-EV and p99-EV errors at 24 EV. Its ceilings prevent regressions on that pattern and do not bound arbitrary edges, images or native rendering. Source fidelity, metadata integrity, gain compression, native color readback and device display remain separate evidence classes. No population-level "99% correct" claim is established by the current selected fixture corpus or CI pass rate.

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

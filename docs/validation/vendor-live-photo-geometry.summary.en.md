# Vendor Live Photo geometry: evidence summary

English | [简体中文](vendor-live-photo-geometry.summary.md)

The [original record](vendor-live-photo-geometry.md) describes a historical Swift implementation and its vendor geometry experiments. It preserves useful observations about ColorOS 16, Samsung, compressed media, and Apple metadata. Its Vision trajectory workflow is not the current Rust execution path.

## Current Rust behavior

The [Live Photo runtime](../../crates/xdremux-runtime/src/live_photo.rs) selects the primary motion-video stream and remuxes it through LibLivePhoto. Geometry is metadata. This path does not render a warp, scale, blur border, or canvas expansion into video pixels.

For the Oplus dialect, the runtime parses OPPO metadata and passes it to [`presentation_geometry`](../../crates/xdremux-motion-photo/src/geometry.rs). It does not call the historical Vision estimator or analyze an auxiliary stream to select this transform.

| Source metadata | Current transform policy |
| --- | --- |
| Metadata version 1 or later | Prefer valid `photo_eis_crop_factor`, then `eis_crop_factor`. If neither is usable, use the existing `0.90` scale. |
| Crop factor greater than 1 | Use its reciprocal; `1.11` becomes `1 / 1.11`. |
| Positive crop factor at most 1 | Keep the value; `0.90` remains `0.90`. |
| Invertible `photo_crop_matrix` or `photo_eis_matrix` | Compose their inverses with the scale. |
| Older metadata with matrices and cover-frame PTS | Select the matrix nearest the cover timestamp. Use its inverse when invertible; otherwise retain the selected matrix before normalization. |
| Missing usable transform or normalized near-identity | Do not emit a transform. |

The normalization check rejects non-finite input values or an unusable denominator. Invalid crop-factor values fall through to the next supported source or the compatibility default. They do not, by themselves, fail the conversion.

When a transform exists, reference dimensions come from valid metadata `video_width` and `video_height`. Both must be positive and at most 16,777,216. This is a source-metadata coordinate space, not an automatic substitution of the full still-image dimensions.

Current ownership and pair-publication rules are documented in [LibLivePhoto integration](../liblivephoto-integration.en.md). Consult the linked implementation and its tests when changing geometry policy.

## Historical observations to retain

The original record distinguishes the primary ColorOS 16 motion stream from a second analysis-only stream. It also records that the tested Samsung resources did not justify inferring another video stream from BMFF-like bytes.

Its Swift experiment used a common `1920×1440` analysis space, a `±0.12 s` cover window, and a 60% trajectory-agreement threshold. It recorded a local ColorOS 16 example with an approximately `1.36612 s` cover time and a measured transform matrix. The exact matrix, fixture identity, and readback observations remain in the original file.

Those measurements describe that experiment. They do not establish that the current Rust product runs the same estimator, applies its private vitality flag, or reproduces the same matrix.

## Evidence requirements

A structurally plausible private payload is insufficient evidence for writing it. The historical record left the Vision-to-Apple coordinate contract for Track 4 and still-processing MakerNote metadata unresolved. Preserve those limits when evaluating a new implementation.

Compressed-sample equality, container parsing, and PhotoKit pair construction prove different properties from Photos visual behavior. Horizontal swipe blur, geometry movement, and cover settling require observations on the applicable device and Photos version.

Keep the original body as historical evidence. Collect new source/output hashes, environment details, commands, and expected/actual results for any new geometry claim. Follow the [validation guide](README.en.md) for current evidence requirements.

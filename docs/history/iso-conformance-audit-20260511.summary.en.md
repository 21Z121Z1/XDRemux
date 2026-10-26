# ISO conformance audit summary: 2026-05-11

English | [简体中文](iso-conformance-audit-20260511.summary.md)

The [original audit](iso-conformance-audit-20260511.md) records the Python and Swift implementations on 2026-05-11. This summary explains its scope. It does not certify the current Rust implementation or repeat the original tests.

## Recorded scope and findings

The audit examined HDR Gain Map and HEIF representation. It covered ISO 21496-1 metadata, item relationships, `tmap` metadata, color properties, bit depth, orientation, and Apple ImageIO compatibility.

The original summary used these labels:

| Historical label | Count | Recorded meaning |
| --- | ---: | --- |
| `RESOLVED` | 2 | Python `hdrgm` XMP embedding and Swift HDRCapacity mapping were corrected. |
| `COMPLIANT` | 9 | The audit accepted the listed container, color, bit-depth, and orientation checks. |
| `SHOULD GAP` | 1 | The Gain Map used half resolution rather than the base image dimensions. |
| `COMPAT CHOICE` | 1 | The Python output retained the 62-byte Apple-compatible `tmap` payload. |

These are the audit's classifications, not a new standards assessment. Consult its complete tables and code excerpts for each conclusion.

## Compatibility result and limit

The tested Apple-compatible 62-byte payload produced a Core Image headroom of `2.4211792945861816` in the recorded normal and passthrough outputs. The strict reference representation used a 65-byte single-channel or 145-byte three-channel `ToneMapImage` payload. Those sizes include the one-byte version field.

The record reports headroom `1.0` or a loading failure for strict-reference outputs in the tested environment. It therefore retained the Apple-compatible payload while preserving the strict reference helper and other container checks.

This result distinguishes standards representation from one consumer's behavior. It does not prove that the strict representation always fails or that the compatibility representation satisfies every standard requirement.

## Use the record

Use the original file for exact historical measurements, commands, and implementation details. Paths such as `python/` and `swift/XDRemux.swift` identify the former architecture.

For current ownership and behavior, use the [architecture guide](../architecture.en.md), current source, and current tests. The [validation guide](../validation/README.en.md) defines how to collect new evidence.

Create a new dated record for a new conformance study. Preserve the 2026-05-11 body, including its measurements, limitations, and compatibility decisions.

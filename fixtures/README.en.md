# Real Media Fixtures

English | [简体中文](README.md)

This directory contains immutable real-device media inputs used to validate XDRemux's portable Rust implementation and migration oracles.

## Layout

- `motion-photo/<vendor>/...` contains Android Motion Photo inputs across JPEG and HEIC/HEIF layouts.
- `proxdr/<vendor>/<device>/...` contains original vendor ProXDR HEIC inputs used to exercise HDR extraction, family detection, Gain Map reconstruction, HEIF assembly, and CLI conversion.

Paths are part of the fixture contract. Prefer stable capability-oriented names over capture timestamps. A vendor or device directory records the provenance of a test input; it is not a product allow-list.

## Identity contract

Media files are committed byte-for-byte. `SHA256SUMS` is the canonical identity manifest for every versioned real-media fixture in this directory.

A strict real-fixture gate must reject a file whose bytes do not match its recorded digest. Do not rewrite a source fixture to normalize metadata, orientation, container layout, vendor tails, or embedded resources when those bytes are part of the behavior being tested.

## Data retained in fixtures

A real fixture can contain EXIF, capture timestamps, vendor metadata, embedded motion-video resources, HDR Gain Maps, portrait data, orientation data, local-HDR metadata, and other source payloads needed by parsers and validators. These are part of the original test input.

Do not assume that a committed real photo is sanitized merely because it is stored under `fixtures/`.

## Current coverage

The Motion Photo corpus covers Samsung, Xiaomi, OPPO, and vivo samples, including JPEG and HEIC/HEIF containers.

The ProXDR corpus currently covers OPPO Find X6 Pro LHDR v1, Find X7 Ultra LHDR v2 including XPAN, and Find X9 Ultra UHDR samples including high-resolution, Master, and Portrait captures. These fixtures are intended to expose format and product-policy differences, not to imply support for every capture mode on every device.

## Generated output

Generated ISO HDR HEIC and Live Photo HEIC/MOV files are temporary test or workflow artifacts. Do not commit converted output here unless a future test explicitly defines it as a versioned golden artifact.

## Adding a fixture

When adding a real fixture:

1. Confirm that it can be published in the repository.
2. Commit the original file without byte modification.
3. Place it under the appropriate capability/vendor/device hierarchy.
4. Add its SHA-256 digest to `SHA256SUMS`.
5. Add or update a test that states exactly what the fixture proves.
6. Do not infer unrelated device support from one fixture.

The repository testing policy is in [docs/quality/testing.en.md](../docs/quality/testing.en.md).

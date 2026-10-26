# Real media fixtures

English | [简体中文](README.md)

This directory stores original device media for tests of the portable Rust implementation and Apple framework consumers. The files are immutable test inputs. A sample's presence does not establish support for every capture mode on its device.

## Find a fixture

| Path | Test purpose |
| --- | --- |
| `motion-photo/<vendor>/...` | Android Motion Photos with JPEG and HEIC/HEIF layouts. |
| `proxdr/<vendor>/<device>/...` | Original ProXDR HEIC files for HDR extraction, format-family detection, Gain Map reconstruction, HEIF assembly, and CLI conversion. |

Paths are part of the test contract. Prefer names that identify a stable capability or layout instead of a capture timestamp. Vendor and device directories record provenance. They are not product allow-lists.

## Preserve file identity

Media files retain their original bytes. [`SHA256SUMS`](SHA256SUMS) is the canonical identity manifest for every versioned real-media file in this directory.

A strict fixture check must reject a file whose digest differs from the manifest. Do not normalize source metadata, orientation, container layout, vendor tails, or embedded resources when those bytes define the tested behavior.

## Check data before publication

Real fixtures can retain Exif, capture times, vendor metadata, embedded video, Gain Maps, portrait resources, orientation, and local-HDR metadata. Parsers and validators can depend on these original payloads.

Do not assume that a photo has been sanitized because it is under `fixtures/`. Confirm that a new sample can be published before adding it.

## Current coverage

Motion Photo fixtures include Samsung, Xiaomi, OPPO, and vivo samples in JPEG and HEIC/HEIF containers.

ProXDR fixtures include these OPPO inputs:

- Find X6 Pro with LHDR v1.
- Find X7 Ultra with LHDR v2, including XPAN.
- Find X9 Ultra with UHDR, including high-resolution, Master, and Portrait captures.

These inputs expose differences in formats and product policy. They do not cover every firmware or capture mode on each device.

## Keep generated media separate

Generated ISO HDR HEIC files and Live Photo pairs are temporary test or workflow artifacts. Do not commit them here unless a test explicitly defines a versioned golden output.

## Add a fixture

1. Confirm that the file can be published in this repository.
2. Choose the appropriate capability, vendor, and device path.
3. Commit the original file without changing its bytes.
4. Add its SHA-256 digest to `SHA256SUMS`.
5. Add or update a test that states what the fixture proves.

Do not infer unrelated device support from one fixture. The [testing policy](../docs/quality/testing.en.md) defines evidence and upload limits.

# Real Motion Photo Fixtures

English | [简体中文](README.md)

This directory contains the real Motion Photo inputs used by strict Swift and pure-Python CI gates.

## Identity contract

The media files are committed byte-for-byte as test fixtures.

`SHA256SUMS` is the canonical identity manifest.

A strict fixture test must reject a file whose bytes do not match the recorded digest.

Do not rewrite a fixture to normalize metadata if the test depends on its original container layout.

## Data retained in fixtures

A real fixture can contain:

- EXIF;
- capture timestamps;
- vendor metadata;
- embedded motion-video resources;
- Gain Maps;
- orientation data;
- other source payloads required by the parser or validator.

These resources are part of the test input.

Do not assume that a committed real photo is sanitized only because it is in a test directory.

## Coverage

The current corpus contains multiple Android Motion Photo implementations and both JPEG and HEIC/HEIF layouts.

The public documentation does not use this corpus as a vendor allow-list. A fixture proves behavior for that file structure and test case.

## Generated output

Generated Live Photo HEIC/MOV files are temporary test or workflow artifacts.

Do not commit generated conversion output to this directory unless a future test explicitly defines it as a versioned golden artifact.

## Adding a fixture

When you add a real fixture:

1. Confirm that it can be published in the repository.
2. Add the original file without byte modification.
3. Add its SHA-256 digest to `SHA256SUMS`.
4. Add or update a test that states what the fixture proves.
5. Do not infer unrelated device support from one fixture.

The repository testing policy is in [docs/quality/testing.en.md](../docs/quality/testing.en.md).

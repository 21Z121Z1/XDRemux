# Texture Style research

This is the canonical research line for iOS 27 Texture Style work.

## Scope

- Texture generation remains unsupported. The Rust transport can preserve an existing opaque Texture carrier byte for byte. It does not certify the input's native acceptance or capture provenance.
- This directory and `scripts/research/` keep reusable contracts, validators, and probes for unresolved questions.
- Historical experiment branches can provide provenance, but one-off workflows do not belong in the canonical working tree.

## Audited baseline blockers (2026-10-19)

The audited baseline `f8fcf2067c506e87665e9ea113324db1a57d4cf7` contained three admission probes that were not valid product facts.

1. `augment_texture_style_heif` reuses one producer-backed 2019 `semanticskinmatte` image and descriptor for all twelve 2026 Texture semantic roles. The published role names therefore exceed the semantics proved by the resource.
2. The Texture metadata payload is a fixed native-positive sample. It publishes `HardwareModel = iPhone19,2`, `CaptureType = LF`, `CaptureMode = Still`, and `PortType = PortTypeBack` without deriving those facts from the source.
3. If the source has no Apple MakerNote, the probe synthesizes an `Apple iOS` MakerNote, a source-derived PhotoIdentifier, and Texture tag 84.

These transformations can test container shape and Apple Photos admission. Device admission does not make the claimed source facts or semantic roles true.

The dedicated `research/texture-native-20261019` line removes that fixed generator. `--apple-styles` uses the v15 Photographic Styles payload that it submits to the native semantic consumer. It does not add 2026 Texture metadata, synthesized Texture MakerNotes, or twelve aliases. The separate `preserve_texture_style_heif` API checks local item references and preserves all bytes of an existing carrier. Missing Texture data causes an unsupported error.

This is an offline transport and provenance correction. It is not a general native Texture producer. Read the [Chinese investigation report](REPORT.zh-CN.md), [machine-readable claims](claims.json), [evidence graph](evidence-graph.json), and [native follow-up list](NATIVE_TODO.md) for the tested boundaries. Use `scripts/research/verify_texture_transport.py --out NEW_DIRECTORY` to repeat the offline checks. Apple API experiments run only in the separate PSR research branch.

## Experiment rule

Use the smallest test that can falsify the current hypothesis. Prefer a local repository script and an existing validation path. Do not add a workflow for each hypothesis.

When a GitHub-hosted Apple runner is required, use a narrow `workflow_dispatch` experiment. Record the durable result in code, a contract, or this research record. Remove the temporary workflow after the experiment. Do not use push-triggered probe workflows or depend on old workflow run IDs or short-lived artifacts.

## Acceptance boundary

Structural HEIC checks, ImageIO readback, and repository completion gates can establish offline conformance only. Apple Photos Texture Style admission and editability require device evidence.

Device admission is still insufficient for promotion. The resource content and metadata facts must also match what the container claims.

# Texture Style research

This is the canonical research line for iOS 27 Texture Style work.

## Scope

- The current Rust Texture Style carrier is a research probe. It is not a product contract.
- This directory and `scripts/research/` keep reusable contracts, validators, and probes for unresolved questions.
- Historical experiment branches can provide provenance, but one-off workflows do not belong in the canonical working tree.

## Current promotion blockers

The current carrier contains three useful admission probes that are not valid product facts.

1. `augment_texture_style_heif` reuses one producer-backed 2019 `semanticskinmatte` image and descriptor for all twelve 2026 Texture semantic roles. The published role names therefore exceed the semantics proved by the resource.
2. The Texture metadata payload is a fixed native-positive sample. It publishes `HardwareModel = iPhone19,2`, `CaptureType = LF`, `CaptureMode = Still`, and `PortType = PortTypeBack` without deriving those facts from the source.
3. If the source has no Apple MakerNote, the probe synthesizes an `Apple iOS` MakerNote, a source-derived PhotoIdentifier, and Texture tag 84.

These transformations can test container shape and Apple Photos admission. Device admission does not make the claimed source facts or semantic roles true.

Do not promote this implementation while these probes remain. Product output must preserve or derive producer-backed facts. It must omit unsupported claims or fail closed.

## Experiment rule

Use the smallest test that can falsify the current hypothesis. Prefer a local repository script and an existing validation path. Do not add a workflow for each hypothesis.

When a GitHub-hosted Apple runner is required, use a narrow `workflow_dispatch` experiment. Record the durable result in code, a contract, or this research record. Remove the temporary workflow after the experiment. Do not use push-triggered probe workflows or depend on old workflow run IDs or short-lived artifacts.

## Acceptance boundary

Structural HEIC checks, ImageIO readback, and repository completion gates can establish offline conformance only. Apple Photos Texture Style admission and editability require device evidence.

Device admission is still insufficient for promotion. The resource content and metadata facts must also match what the container claims.

# Texture Style research

This is the canonical research line for iOS 27 Texture Style work.

## Scope

- The current Rust Texture Style carrier is a research probe. It is not a product contract.
- This directory and `scripts/research/` keep reusable contracts, validators, and probes for unresolved questions.
- Historical experiment branches can provide provenance, but one-off workflows do not belong in the canonical working tree.

## Current promotion blocker

`augment_texture_style_heif` currently reuses one producer-backed 2019 `semanticskinmatte` image and descriptor for all twelve 2026 Texture semantic roles.

This is useful for container and Photos-admission experiments. It does not prove that the aliased resources contain the semantics named by those roles. A device UI that accepts the carrier does not prove that claim.

Do not promote this implementation while it relies on those aliases. Promotion requires producer-backed content for every published semantic role, or native evidence that the corresponding role is allowed to reference that exact resource without changing its semantic meaning.

## Experiment rule

Use the smallest test that can falsify the current hypothesis. Prefer a local repository script and an existing validation path. Do not add a workflow for each hypothesis.

When a GitHub-hosted Apple runner is required, use a narrow `workflow_dispatch` experiment. Record the durable result in code, a contract, or this research record. Remove the temporary workflow after the experiment. Do not use push-triggered probe workflows or depend on old workflow run IDs or short-lived artifacts.

## Acceptance boundary

Structural HEIC checks, ImageIO readback, and repository completion gates can establish offline conformance only. Apple Photos Texture Style admission and editability require device evidence.

Device admission is still insufficient for semantic-role promotion. The resource content must also match the role that the container claims.

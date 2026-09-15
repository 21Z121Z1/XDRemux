# Texture Style research

This is the canonical research line for iOS 27 Texture Style work.

## Scope

- Product-facing carrier support lives in the Rust implementation and its normal repository tests.
- This directory and `scripts/research/` keep only reusable research contracts, validators, and probes that are still useful for answering unresolved questions.
- Historical experiment branches are merged into this line for provenance; their one-off GitHub Actions are not part of the canonical working tree.

## Experiment rule

Use the smallest test that can falsify the current hypothesis. Prefer a local/repository script plus an existing validation path. Do not add a new workflow for each hypothesis.

When a GitHub-hosted Apple runner is genuinely required, use a narrowly scoped `workflow_dispatch` experiment, record the durable result in code/contract/documentation, and remove the temporary workflow after the experiment. Avoid push-triggered probe workflows and hard-coded dependencies on old workflow run IDs or short-lived artifacts.

## Acceptance boundary

Structural HEIC checks, ImageIO readback, and repository completion gates can establish offline conformance only. Apple Photos Texture Style admission/editability remains device evidence and must not be inferred from container structure alone.

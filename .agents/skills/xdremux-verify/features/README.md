# XDRemux feature verification map

Use the built CLI named by `XDREMUX_VERIFY_CLI` from the repository root. Read the [launch and doctor steps](../SKILL.md) before a recipe. Public fixture identities come from `fixtures/SHA256SUMS`. Use isolated paths; do not overwrite a user source during verification.

| Feature | Public entry points | Evidence owner |
| --- | --- | --- |
| [Inspect and validate](inspect-validate.md) | `inspect`, `validate`, human/JSON output | CLI and validation contracts |
| [Motion Photo](motion-photo.md) | `convert`, paired still/movie validation | Existing real-fixture and negative runtime tests |
| [Batch and categorize](batch-categorize.md) | `batch`, `categorize`, receipts/dry-run | Existing CLI smoke, batch, provenance and publication tests |
| [HDR and Apple editing](hdr-apple.md) | HDR `convert`, Styles/Portrait/Cinematic flags, app switch | Existing codec, native product, app and device gates |

Read all affected entry points when changing shared policy. A convenient CLI entry point does not establish the app path or batch path. Record the exact feature, inputs, environment, binary identity, result, and missing preconditions for each check. Do not report an unreachable entry point as passed through another surface.

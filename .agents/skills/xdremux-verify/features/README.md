# XDRemux feature verification map

Use the built CLI named by `XDREMUX_VERIFY_CLI` from the repository root. Before following a recipe, read the [build and prerequisite steps](../SKILL.md). Check public fixture identities against `fixtures/SHA256SUMS`. Use separate paths owned by the verification run. Do not overwrite a user source.

## Select the affected entry points

| Feature | Public entry points | Evidence owner |
| --- | --- | --- |
| [Inspect and validate](inspect-validate.md) | `inspect`, `validate`, human/JSON output | CLI and validation contracts |
| [Motion Photo](motion-photo.md) | `convert`, paired still/movie validation | Existing real-fixture and negative runtime tests |
| [Batch and categorize](batch-categorize.md) | `batch`, `categorize`, receipts/dry-run | Existing CLI smoke, batch, provenance and publication tests |
| [HDR and Apple editing](hdr-apple.md) | HDR `convert`, Styles/Portrait/Cinematic flags, app switch | Existing codec, native product, app and device gates |

If shared policy changes, read every affected entry point. A successful CLI command does not establish the app or batch path.

## Record each result

For each check, record the exact feature, inputs, environment, binary identity, result, and missing preconditions. If an entry point is unreachable, report the gap. Do not use another interface to claim that it passed.

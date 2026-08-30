# Tests

English | [简体中文](README.md)

This directory contains Swift package tests, Python repository-policy tests, and reusable validation harnesses.

## Swift tests

Run all Swift package tests from the repository root:

```bash
swift test
```

Main test targets:

| Target | Scope |
| --- | --- |
| `XDRemuxCoreTests` | Conversion models, HEIF/ISO-BMFF behavior, Motion Photo parsing, validation, classification, and file lifecycle. |
| `XDRemuxAppleFeaturesTests` | Live Photo, Photographic Styles, Apple Portrait, native-helper compatibility, and performance contracts. |
| `XDRemuxCLITests` | CLI parsing, batch behavior, Motion Photo routing, and output safety. |

## Python repository tests

Run:

```bash
python3 -m unittest discover -s Tests -v
```

These tests include Python converter behavior and repository policies that inspect Swift source, documentation, fixtures, or architecture.

A source-inspection policy test is static evidence. It is not a replacement for a functional conversion test.

## Validation harnesses

Reusable harnesses are in `Tests/validation/`.

Examples include:

- `verify_swift_cli_sample.py`
- `verify_python_motion_photo_fixtures.py`
- `verify_error_messages.sh`
- `verify_batch_categorize_idempotence.sh`
- `verify_categorization_cross_implementation.py`
- `verify_categorized_batch_outputs.py`
- `verify_apple_feature_artifact_lifecycle.py`
- `verify_macos_app_model_tests.sh`

Use the [regression and real-sample guide](../docs/quality/evals.en.md) to select a harness.

## Real Motion Photo fixtures

The strict Motion Photo corpus is under `fixtures/`, not under `Tests/fixtures/`.

`fixtures/SHA256SUMS` defines the exact identities of the real media files.

`Tests/fixtures/` is for small synthetic or metadata-only fixtures that are safe to regenerate as test data.

## Completion gate tests

`Tests/validation/test_agent_completion_gate.py` tests the repository completion-gate implementation.

The test checks required evidence handling and receipt invalidation behavior.

The production acceptance procedure is documented in [docs/validation/README.en.md](../docs/validation/README.en.md).

## Documentation tests

`Tests/test_public_documentation.py` checks current public-document links and bilingual publication rules.

When you add a new normative technical document, add it to the documentation policy if it is part of the public bilingual set.

Current documentation follows the [technical writing guide](../docs/style-guide.en.md).

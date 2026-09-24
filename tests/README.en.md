# Tests

English | [简体中文](README.md)

This directory contains Rust product-policy tests, Python repository-policy tests, validation-framework self-tests, and small synthetic fixtures.

## Rust tests

Run the canonical product tests from the repository root:

```bash
cargo test --workspace --locked
```

The Swift package contains only the Apple primitive adapter. Build it when running macOS consumer checks:

```bash
swift build --package-path platforms/apple --product xdremux-apple-adapter
```

The public CLI parsing, conversion, batch, Motion Photo, classification, Portrait, Styles, validation, and output-safety tests live in the Rust workspace.

## Python repository tests

Run:

```bash
python3 -m unittest discover -s tests -t . -v
```

These tests cover repository policies, documentation, app architecture, and the optional research/training package. They are not a product conversion implementation.

A source-inspection policy test is static evidence. It is not a replacement for a functional conversion test.

## Validation harnesses

Reusable portable harnesses are in `scripts/validation/`. The macOS app owns its model-test launcher in `apps/macos/XDRemuxApp/scripts/`.

`tests/validation/` contains Python self-tests for the validation framework, workflow configuration, and repository layout; it does not contain shell launchers.

Examples include:

- `scripts/validation/check_rust_motion_photo_real_fixtures.sh`
- `scripts/validation/verify_error_messages.sh`
- `scripts/validation/verify_batch_categorize_idempotence.sh`
- `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh`

Use the [regression and real-sample guide](../docs/quality/evals.en.md) to select a harness.

## Real Motion Photo fixtures

The strict Motion Photo corpus is under `fixtures/`, not under `tests/fixtures/`.

`fixtures/SHA256SUMS` defines the exact identities of the real media files.

`tests/fixtures/` is for small synthetic or metadata-only fixtures that are safe to regenerate as test data.

## Completion gate tests

`tests/validation/test_agent_completion_gate.py` tests the repository completion-gate implementation.

The test checks required evidence handling and receipt invalidation behavior.

The production acceptance procedure is documented in [docs/validation/README.en.md](../docs/validation/README.en.md).

## Documentation tests

`tests/test_public_documentation.py` checks current public-document links and bilingual publication rules.

When you add a new normative technical document, add it to the documentation policy if it is part of the public bilingual set.

Current documentation follows the [technical writing guide](../docs/style-guide.en.md).

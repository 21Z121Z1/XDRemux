# Tests

English | [简体中文](README.md)

This directory contains Python regressions, product and repository policy checks, verification self-tests, and small synthetic fixtures. Rust tests live with their crates. Run the following commands from the repository root.

## Run Rust product tests

```bash
cargo test --workspace --locked
```

The Rust workspace tests CLI parsing, conversion, batch operations, Motion Photo, classification, Portrait, Styles, validation, and output safety.

For macOS consumer checks, build the Apple framework adapter:

```bash
swift build --product xdremux-apple-adapter
```

The Swift package publishes the adapter executable and contains its native support targets. It does not publish another conversion product.

## Run Python repository tests

```bash
python3 -m unittest discover -s tests -t . -v
```

These tests cover repository rules, documentation, app architecture, and optional research and training code. They do not implement product conversion. A test that inspects source text provides static evidence. It cannot replace a functional conversion test.

## Select a validation script

Reusable portable checks are in `scripts/validation/`. The app model-test launcher is in `apps/macos/XDRemuxApp/scripts/`.

| Script | Purpose |
| --- | --- |
| `scripts/validation/check_rust_motion_photo_real_fixtures.sh` | Convert and validate the real Motion Photo corpus. |
| `scripts/validation/verify_error_messages.sh` | Check selected CLI help and error results. |
| `scripts/validation/verify_batch_categorize_idempotence.sh` | Check repeated categorized batch behavior. |
| `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh` | Build and run macOS app model tests. |

`tests/validation/` contains Python self-tests for the completion gate, workflow configuration, and repository layout. Shell launchers do not belong there. See the [regression and real-media guide](../docs/quality/evals.en.md) to select checks.

## Select the correct fixture location

The real Motion Photo corpus is in `fixtures/`. Its exact file identities are in `fixtures/SHA256SUMS`.

Use `tests/fixtures/` for small synthetic or metadata-only inputs that can be regenerated as test data. Do not move real media there.

## Verify the completion gate

`tests/validation/test_agent_completion_gate.py` checks required evidence handling and receipt invalidation. The [validation runbook](../docs/validation/README.en.md) explains how to use the gate for a committed change.

## Check documentation

`tests/test_public_documentation.py` checks the bilingual publication policy and required document references. `tests/validation/test_repository_layout.py` checks local links and their exact case.

If a new normative document joins the public bilingual set, include it in the documentation policy checks. Follow the [technical writing guide](../docs/style-guide.en.md).

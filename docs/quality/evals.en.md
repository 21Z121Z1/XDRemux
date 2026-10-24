# Verify regressions and real media

English | [简体中文](evals.md)

Use the existing validation scripts when a change needs evidence beyond a unit test. The [testing policy](testing.en.md) defines the required evidence for each kind of change.

## Check versioned Motion Photos

The repository contains real Motion Photo fixtures under `fixtures/`. Before conversion, the strict Rust check compares each input with `fixtures/SHA256SUMS`. A digest mismatch fails the check.

The corpus covers several JPEG and HEIC/HEIF layouts. Read the workflow and test source for the current assertions. Across the applicable unit, fixture, and native checks, verify these contracts:

- The parser identifies Motion Photo resource boundaries.
- The selected cover time maps to Apple `still-image-time`.
- The output still and MOV have the expected shared Live Photo identifier.
- The output retains the source Gain Map when the conversion requires it.
- Normal compressed-sample passthrough preserves encoded video and any source audio samples.
- Publication does not silently reuse a pair whose source provenance is unknown.
- Applicable macOS checks load generated pairs through the tested Apple framework interface.

## Select an existing script

Portable validation scripts are in `scripts/validation/`. The app's model-test launcher is in `apps/macos/XDRemuxApp/scripts/`.

| Script | Purpose |
| --- | --- |
| `scripts/validation/check_rust_motion_photo_real_fixtures.sh` | Convert every versioned Motion Photo through the Rust CLI and validate both output members. |
| `scripts/validation/verify_error_messages.sh` | Check selected help and error behavior through the real CLI. |
| `scripts/validation/verify_batch_categorize_idempotence.sh` | Check repeated batch conversion with categorized output. |
| `scripts/validation/verify_validate_only_harness.sh` | Check validation-only behavior. |
| `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh` | Build and run app model tests on macOS. |

Choose evidence for the part that changed:

| Affected behavior | Checks |
| --- | --- |
| Gain Map encoding or HEIF structure | HDR validation scripts and a representative real input. |
| Motion Photo parsing or writing | Rust unit tests and the strict real-fixture check. |
| Batch provenance or output safety | Output-collision and checkpoint regressions. |
| Classification | Rust classification tests and output-layout checks. |
| App state | App model tests. |
| Photos behavior | Native framework or device evidence, in addition to structural checks. |

## Inspect current performance reports

[`benchmark_rust_product.py`](../../scripts/performance/benchmark_rust_product.py) writes schema-v2 reports incrementally. Each measured conversion records elapsed time, peak RSS, source identities, completed work, and CLI output validation. Hash calculation and validation occur outside the timed conversion.

If conversion or validation fails, the report retains the failure and the observed samples. It does not report a successful summary. The batch workload requires three converted inputs, with no failed or reused items. Each published output must pass `xdremux validate`.

[`check_performance_budget.py`](../../scripts/performance/check_performance_budget.py) requires at least five completed measurements for each workload in a current schema-v2 report. It recomputes the median, p95, and peak RSS from those samples before it applies the budget.

The report must identify the executable, `Cargo.lock`, and execution. Execution identity includes the host, CPU count, and explicit hosted-run fields. A local field that was not recorded must be `null`. Live Photo samples retain the identities and validation results for both same-stem members.

The check rejects missing observations, inconsistent summaries, incomplete work, failed validation, and current reports downgraded to summary-only schema v1. If collection fails before summary statistics exist, the workflow summary still shows collection status, sample counts, and errors.

## Interpret historical measurements

Historical baseline files retain schema v1 and their original values. Use `--historical-replay` only to inspect archived v1 run reports under their original summary contract. That mode does not provide current CI evidence.

A passed budget does not establish why performance changed. A speed or memory comparison needs matched work, appropriate measurements on the same host, and evidence of the limiting work.

## Limit the conclusion to the evidence

One script cannot prove all user-visible behavior. Structural validity does not establish visual equivalence. A fixture corpus establishes behavior for those files, not every firmware, device, or capture mode.

Do not require byte-identical golden output for every conversion. Valid outputs can include generated identifiers or encoding choices that depend on the framework.

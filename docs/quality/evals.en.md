# Regression and Real-Sample Verification

English | [简体中文](evals.md)

Use a reusable harness when a change needs evidence beyond a unit test.

The [testing policy](testing.en.md) defines which evidence class is required.

## Versioned Motion Photo gates

The repository contains real Motion Photo fixtures under `fixtures/`.

Current strict gates include Swift and pure-Python paths. They verify input identity before conversion and reject a fixture whose bytes do not match `fixtures/SHA256SUMS`.

The fixture gates cover multiple JPEG and HEIC/HEIF container layouts. The gate names and exact assertions can change with the implementation, so use the workflow and test source as the final authority.

Important current assertions include:

- Motion Photo resource boundaries are parseable.
- The selected cover time can be mapped to Apple `still-image-time`.
- The output still and MOV share the expected Live Photo asset identity.
- Source Gain Map presence is preserved when required.
- Compressed video samples are preserved by the normal passthrough path.
- Compressed audio samples are preserved when source audio is present.
- Output publication does not silently reuse a valid pair from unknown source provenance.
- macOS validation can load applicable generated pairs through the tested Apple framework path.

## Reusable validation harnesses

`Tests/validation/` contains reusable scripts.

| Harness | Purpose |
| --- | --- |
| `verify_swift_cli_sample.py` | Run or validate a Swift HDR conversion and inspect the expected Gain Map pixel format. |
| `verify_error_messages.sh` | Check selected help and error contracts through the real Swift binary. |
| `verify_batch_categorize_idempotence.sh` | Check repeated categorized batch behavior. |
| `verify_validate_only_harness.sh` | Check validation-only behavior. |
| `verify_categorization_cross_implementation.py` | Compare Swift and Python classification results. |
| `verify_categorized_batch_outputs.py` | Check the categorized directory projection. |
| `verify_apple_feature_artifact_lifecycle.py` | Check Apple-feature temporary artifact policy. |
| `verify_macos_app_model_tests.sh` | Build and run the macOS app model tests. |
| `verify_python_motion_photo_fixtures.py` | Convert and validate the versioned Motion Photo corpus with Python. |

## Choose evidence by affected path

- Gain Map encoding or HEIF structure: use the HDR validation harness and a representative real input.
- Motion Photo parser or writer: use the strict fixture gates and the targeted Swift/Python tests.
- Batch provenance or output safety: use the output-collision and checkpoint regressions.
- Classification: use both cross-implementation and output-layout checks when both implementations are affected.
- App state: use the macOS app model tests.
- Apple Photos behavior: use native-framework or device evidence in addition to structural checks.

## Known limits

No single harness proves every user-visible behavior.

Bit-for-bit golden output is not a universal requirement because some valid outputs contain generated identifiers or framework-dependent encoding choices.

Structural validation does not prove visual equivalence.

A fixture corpus proves behavior for those files. It does not prove every firmware, device, or capture mode.

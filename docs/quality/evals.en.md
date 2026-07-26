# Regression and real-sample verification

English | [简体中文](evals.md)

So that "I looked at it and it seemed fine" is never the evidence. This lists the reusable verification harnesses and what each one actually proves. For the rules, see the [testing policy](testing.en.md).

## Reusable harnesses

All of these live in `Tests/validation/` and can be used directly as a check in a completion-gate plan.

| Harness | What it proves |
| --- | --- |
| `verify_swift_cli_sample.py` | Converts one real photo end to end and asserts the gain-map pixel format through ImageIO. `--require-compressed-primary-preserved` additionally asserts the primary image bytes were untouched; `--validate-only` inspects an existing output without converting |
| `verify_error_messages.sh` | Checks help text and error text through the real binary: re-converting an output, a non-ProXDR input, and the length of a batch failure line |
| `verify_batch_categorize_idempotence.sh` | Runs `batch --categorize` twice over one directory; the second run must skip everything rather than re-scanning its own output |
| `verify_validate_only_harness.sh` | `--validate-only` on a match, a mismatch, and a misuse |
| `verify_categorization_cross_implementation.py` | The Swift and Python implementations categorize identically |
| `verify_categorized_batch_outputs.py` | The directory structure a categorized batch produces |
| `verify_apple_feature_artifact_lifecycle.py` | Apple feature intermediates are cleaned up or retained as intended |
| `verify_macos_app_model_tests.sh` | Builds and runs the macOS app's model tests |

## Choosing one

- Changed gain-map encoding or container writing → `verify_swift_cli_sample.py` with the expected pixel format.
- Changed text users see → `verify_error_messages.sh`.
- Changed batch enumeration or resume logic → `verify_batch_categorize_idempotence.sh`.
- Changed categorization → the two categorization harnesses.
- Changed the app's view model → `verify_macos_app_model_tests.sh`.

Real samples are not in the repository, so plans must give absolute paths to them.

## Known gaps

1. There is no golden-output comparison. The assertions are structural — pixel format, byte preservation, exit codes — not bit-identical output hashes.
2. Real-sample coverage is thin; only a few models and shooting modes are represented.
3. Nothing automatically proves a Photographic Styles output is genuinely editable in Photos. The checks reach container structure only.
4. OPPO Gallery behaviour needs a real device and is in no automated path.

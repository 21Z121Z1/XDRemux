# Methods for native-response and calibration experiments

English | [简体中文](methodology.md)

This document preserves methods and limitations from the historical solver experiments. It does not reproduce the private experiments or define another converter. The [model card](models/UniversalPhotographicStyleStateNet.model-card.en.md) identifies the public model and its historical measurements.

The earlier convergence ledger is absent from this tree. Claims about retired references require the original ledger and source identities. This account cannot replace that evidence.

## Separate the data cohorts

The historical native dataset contained 603 samples from 472 capture sessions. The training, calibration, and heldout splits contained 417, 89, and 97 samples. These are sample counts, not session counts. The account reports session-disjoint splits, but the private manifest and source bytes are absent from Git.

The native-response cohort used the lexicographically smallest source SHA-256 in each existing split and device group. Selection did not inspect image quality or metrics. The reported cohort covered four devices, with one calibration session and a different heldout session for each device.

For a new run, hash the actual bytes. Reject missing files, repeated identities, and sessions shared between splits. Record the dataset, model, checkpoint, code, and OS identities in a new immutable manifest. The retired selector trusted inventory hashes and overwrote its destination. Those operations are not a reproducible identity check.

The prospective OPPO cohort excluded four historical A/B scenes and every SHA-256 in their earlier audit. It then selected one available image per device group by source hash. Five device groups were reported. The inventory had already been inspected without labels, so "prospective consumer cohort" does not mean an unseen image distribution.

Exclude both scene identities and byte identities. Record earlier exposure to the inventory. Do not select new images after examining model responses. A path that exists does not prove that its contents match the recorded hash.

Historical A/B scenes influenced model and scale selection. They are therefore **unlocked**. The out-of-distribution trial used calibration-p95 uncertainty as an admission threshold. That threshold measures coverage, not native rendering accuracy. DNG previews are not decoded RAW observations.

Current [training.py](training.py) and [inference.py](inference.py) require decoded optional inputs and checked byte identities. They do not infer input availability from file extensions or marker strings.

## Freeze the response matrix

The historical matrix used these eight states:

| State | Role |
| --- | --- |
| `disabled` | Disabled-style reference. |
| `neutral` | Neutral-style reference. |
| `tone_+1` | Positive Tone endpoint. |
| `tone_-1` | Negative Tone endpoint. |
| `color_+1` | Positive Color endpoint. |
| `color_-1` | Negative Color endpoint. |
| `tc100_mid` | Historical combined-control state. |
| `tc100_plus` | Historical combined-control state. |

Freeze the complete matrix before rendering. Preserve the original and candidate carriers, requests, returned pixels, and source, candidate, model, and checkpoint hashes. Record the framework, OS, commands, exit status, and errors.

A complete Styles resource graph, a finite proxy response, and a native-renderer response are different evidence. Keep color space, orientation, dimensions, and bit-depth interpretation fixed. Do not silently convert incompatible pixel representations.

The alpha grid was `[0, .25, .5, .625, .75, 1]`, with baseline `.625`. Proposed residual gains were `[.5, .75, 1, 1.25, 1.5]`. Calibration selected a candidate only when all these conditions held:

- Improvement was at least 1%.
- No device regressed by more than 10%.
- Missing responses, failed responses, and direction reversals did not increase.
- Every required state and cohort was present.

The score averaged encoded-RGB RMSE across states. It was not a pooled linear-light error.

## Preserve failures in the reports

The old alpha summarizer could label cached heldout baseline pixels with `chosenAlpha`, even when selection chose another alpha. It also averaged incomplete state sets and tried multiple filename fallbacks. Another summarizer could report complete calibration in Markdown while its JSON reported incomplete calibration. These report writers are retired.

A missing candidate or state must remain missing. Candidate measurements must use that exact candidate, not a baseline cache. Persist selected parameters and their provenance before evaluating heldout data. Machine-readable results and prose must report the same decision.

A no-cache control compared alpha 0, .625, and 1. It used distinct Key 1 inputs and carriers, with separate `neutral` and `tone_plus` requests and outputs. Equal pixels can show that a consumer ignores a change. Unequal hashes are not an acceptance requirement.

Different filenames do not prove different requests or executions. Hash the request-to-carrier sequence and retain the renderer output identity. Do not replace an unavailable renderer with identity output or a semantic proxy.

The historical account of Issue #32 describes private-ABI changes on macOS 27.0 `26A5416b` with Xcode 27 `27A5218g`. The old `PLPhotoEditSource` initializer lost an image argument. `_NUStyleTransferApplyProcessor` gained a displacement argument before color space. Those retired Swift callers are not the current Rust product or narrow adapter.

Before calling a private method in a local experiment, inspect and validate its exact signature. A timeout, missing selector, or ABI mismatch is blocked evidence. It is not a successful identity fallback. Current research CI is defined separately in [research.yml](../../.github/workflows/research.yml), including its pinned tone-response experiment. This historical account does not certify those runs.

## Apply a frozen correction

[calibration.py](calibration.py) preserves the useful contract from `evaluate_selfpair_lowdim_adapter.py`. It fits thirty normalized polynomial/output-channel biases shared across all spatial and plane positions. The fit uses observed blocks and positive ridge regularization.

Application uses only stored coefficients. Its interface has no target or mask argument. Fitting and application leave their inputs unchanged. Both check finite values, array shapes, positive scales, bounded size, and arithmetic overflow.

Fit on the designated training cohort:

```python
from research.oppo_styles.calibration import ChannelBias, fit_channel_bias, apply_channel_bias

fitted = fit_channel_bias(train_prediction, train_target, train_mask, scales, ridge=10.0)
frozen_record = fitted.to_dict()
```

Bind the saved record to code, cohort, and model hashes. Use calibration for selection without fitting those coefficients again. After selection is recorded, restore the coefficients for heldout application:

```python
restored = ChannelBias.from_dict(frozen_record)
heldout_prediction = apply_channel_bias(heldout_base_prediction, scales, restored)
```

The retired script saved biases but ignored them in its heldout channel-ridge branch. That branch read heldout targets and fitted again, contrary to its own comment. The maintained application interface cannot accept those labels.

Tests change labels after the fit, vary spatial and plane dimensions, restore saved coefficients, and reject non-finite or overflowing inputs. They prove the frozen-coefficient contract, not native accuracy.

## Distinguish proposed estimators from executed fits

Other historical families remain research hypotheses:

| Family | Recorded search space |
| --- | --- |
| Alpha blend | Increments of .05. |
| Global correction | Residual gain .85–1.15 and normalized bias −.02–.02. |
| Conditional blend | Device/profile or luminance-dependent blends. |
| Shared correction | Shared bias or channel bias. |
| Device correction | One bias parameter with ridge 1/3/10/30/100/300. |
| Session residual PCA | Rank 1/2/3, image RGB means and deviations, ridge 100. |
| kNN | 3/5 neighbors with .25/.5/1 shrink. |

Training sessions supplied fits. Calibration selected the family and promotion decision. Unknown device profiles used a fixed `.625` fallback. The historical bootstrap used capture-session clusters, 20,000 draws, and seed 260819. Independent pixels are not independent samples for these confidence intervals. Heldout data must neither fit coefficients nor select a family.

The 17 Pro scripts emitted rejection strings unconditionally. A candidate named "30-parameter term/channel gain" was an identity operation, and its "bias" used one scalar. Some profile reconstruction paths implemented only device-alpha variants. Their names do not prove that the declared families were correctly fitted or evaluated.

A renewed estimator needs its actual mathematical implementation, explicit fitted state, source-order and session checks, and an executed promotion rule. Hard-coded verdicts are not acceptance evidence.

## Interpret training and export attempts

The maintained training package distinguishes paired input and single-image self-pair through `TrainingInputMode`. Self-pair changes observations, not native targets or cached bytes.

A historical heads-only experiment combined true-pair and self-pair Huber losses with a detached true-prediction consistency target. It used a very small learning rate, permissive checkpoint loading, and reusable output directories without safeguards. Its checkpoint/report wrapper is retired. A repeat needs checked provenance, restricted loading, fresh run directories, finite optimization, fixed splits, and calibration-selected checkpoints.

| Historical attempt | Reported normalized heldout MAE | Comparison |
| --- | ---: | --- |
| Short self-pair candidate 1 | .80226 | Worse than self-pair baseline .78220. |
| Short self-pair candidate 2 | .78751 | Worse than self-pair baseline .78220. |
| Universal followed by frozen paired model | .82674 | Worse than standalone universal .82233. |

Neither short self-pair candidate was promoted. The cascade used an unstyled thumbnail predicted by the universal model. The oracle used a real native disabled thumbnail. These inputs are different. Oracle accuracy is not a runtime capability. The figures are private historical measurements, not new CI results.

The former Core ML comparison allowed .02 error for Key 1 and light maps, up to 2 for GTC byte values, and .1 for scalars. A repeat needs the exact checkpoint, normalization, layout, source bytes, Core ML package, compute configuration, and orientation-aware serialization. The package alone does not supply the private inputs.

Current [CoreMLProbe.swift](CoreMLProbe.swift) compiles the public package and executes synthetic inputs with finite-value and shape checks. It uses `.cpuOnly`. It does not establish Python-output equivalence, native-renderer equivalence, or Neural Engine placement.

## Requirements for product use

A research result requires fixed identities and splits, a complete response matrix, and no hidden fallback or label leakage. Controls must include meaningful identity, shuffled, paired, and ablated cases. Device limits and source-conditioned resources must be explicit and reproducible.

Product integration also requires the existing Rust intent, capability, and publication owners. Metadata must have the correct role, and native consumers must be tested. Claims of Photos editability require actual import, edit, save, and reopen evidence. A preserved hypothesis is not automatically a product defect or a shipped feature.

# Native-consumer and low-dimensional experiments

English | [简体中文](methodology.md)

This preserves the solver branch's **research methodology**, not an alternate converter or a claim that private experiments have been reproduced. The previously referenced convergence ledger is not present in this tree. Recover the original ledger and source identities before relying on retired-ref provenance; this methodology is not a substitute for those records. The unchanged public model and historical measurements are documented separately in its [model card](models/UniversalPhotographicStyleStateNet.model-card.en.md).

## Data boundaries and prospective cohorts

The historical native dataset reported 603 samples in 472 capture sessions; 417/89/97 are **sample** counts in session-disjoint train/calibration/heldout splits, not three session counts. The private manifest and source bytes are absent from Git. Reproduction requires those inputs and their identities; the public model package cannot establish the claimed split by itself.

The native-consumer freezer selected the lexicographically smallest source SHA-256 within each existing split/device, without inspecting image quality or metrics: four devices × one calibration session and four different heldout sessions. A proper replay must hash the actual bytes, reject missing/repeated/session-overlapping samples, pin dataset/model/checkpoint/code/OS identities, and publish a new immutable manifest. The old script trusted inventory hashes and overwrote its destination; those mechanics are not retained.

The prospective OPPO freezer excluded four historical A/B scenes and every SHA-256 found in their earlier audit, then selected one available image per canonical device by source hash. Five device groups were reported. The inventory had already been observed without labels: “prospective consumer cohort” is not an untouched image-distribution claim. Exclude both scene and byte identities, document previous exposure, and never resample after looking at responses. A path's existence does not establish its recorded hash.

Historical A/B scenes were used for model/scale selection and are explicitly **unlocked**. The OOD trial's calibration-p95 uncertainty admission measures coverage, not native rendering accuracy. DNG previews do not constitute decoded RAW observations. Maintained `training.py` and `inference.py` enforce the decoded-modality and byte-identity contracts instead of restoring marker/suffix heuristics.

## Native response matrix and causal controls

Freeze all eight states before rendering: `disabled`, `neutral`, `tone_+1`, `tone_-1`, `color_+1`, `color_-1`, `tc100_mid`, `tc100_plus`. Preserve the original and candidate carriers, state requests, actual returned pixels, source/candidate/model/checkpoint hashes, framework/OS identity, commands, exit status and errors. A structural Styles graph, a finite proxy response, and an actual private-renderer response are different evidence.

The native alpha grid was `[0, .25, .5, .625, .75, 1]`, with current baseline `.625`; proposed residual gains were `[.5, .75, 1, 1.25, 1.5]`. Selection used calibration only: at least 1% improvement, no device regression above 10%, and no increase in missing/failed responses or direction reversals. All required states/cohorts must actually be present before making that comparison. The score was a mean of per-state encoded-RGB RMSE values, not a pooled linear-light error. Keep color-space, orientation, dimensions and bit-depth interpretation fixed; do not silently convert incompatible pixel contracts.

The old alpha summarizer had an invalid generalization: it labeled heldout baseline cached pixels with `chosenAlpha` even when another alpha could be selected. It also averaged partial state sets and used multiple filename fallbacks. The old consumer summarizer's Markdown conclusion could say calibration was complete even when its JSON said otherwise. **These report writers are intentionally retired, not transplanted.** A missing candidate/state stays missing; candidate results must reference that exact candidate, never a baseline cache. Write heldout results only after persisting the selected parameters and their provenance, and make the machine-readable and human-readable decision agree.

A no-cache control compared alpha 0, .625 and 1: distinct key1 inputs, distinct carriers and separate requests/output paths for `neutral` and `tone_plus`. Identical rendered pixels can legitimately show consumer insensitivity; unequal pixel hashes are not an acceptance requirement. Conversely, different filenames do not prove different requests or execution. Hash the actual request-to-carrier chain and retain the renderer's output identity. Never replace an unavailable renderer with identity output or a semantic proxy.

Issue #32 described private-ABI drift on macOS 27.0 `26A5416b` / Xcode 27 `27A5218g`: the old `PLPhotoEditSource` initializer lost an image argument, and `_NUStyleTransferApplyProcessor` gained displacement before color space. Those old Swift producer/oracle callers are not the Rust product or current narrow adapter. Future local consumer harnesses must inventory and validate exact method signatures before calling; a timeout, missing selector or ABI mismatch is blocked evidence, not successful identity fallback. No firmware acquisition or private renderer is enabled in CI by this convergence.

## Frozen low-dimensional correction

The useful contract from `evaluate_selfpair_lowdim_adapter.py` is now `calibration.py`: fit thirty normalized polynomial/output-channel biases shared across space and planes, serialize them, and apply **only those stored values**. The fit uses observed blocks and positive ridge regularization; application has no target or mask argument. Neither call mutates its inputs. Finite values, array shapes, scale positivity, bounded size and arithmetic overflow are checked.

```python
from research.oppo_styles.calibration import ChannelBias, fit_channel_bias, apply_channel_bias

# Fit from a designated training cohort; calibrate/select without refitting it.
fitted = fit_channel_bias(train_prediction, train_target, train_mask, scales, ridge=10.0)
frozen_record = fitted.to_dict()  # bind this record to code/cohort/model hashes
# After selection has been persisted, restore it for heldout application.
restored = ChannelBias.from_dict(frozen_record)
heldout_prediction = apply_channel_bias(heldout_base_prediction, scales, restored)
```

The old script saved fitted biases but ignored them in its heldout channel-ridge branch, calling a function that read heldout targets and fitted again. That violates its “never refit on heldout” comment. The new application interface cannot do this. Tests vary labels after freezing, change spatial/plane dimensions, round-trip the record and reject non-finite/overflowing inputs. These tests establish a frozen-parameter contract, not native accuracy.

Other historical families remain hypotheses rather than supported estimators: blend alpha in .05 increments; global residual gains .85–1.15 and normalized biases −.02–.02; per-device/profile or luminance blends; shared/channel bias; one-parameter device bias with ridge 1/3/10/30/100/300; rank-1/2/3 session-mean residual PCA with image RGB means/deviations and ridge 100; and kNN 3/5 with .25/.5/1 shrink. Train sessions supplied fits; calibration supplied selection/promotion, with fixed `.625` fallback for unknown device profiles. Use session-cluster bootstrap (the old experiment used 20,000 draws, seed 260819), not independent-pixel confidence intervals. Independent heldout must not fit coefficients or choose the family.

The 17 Pro scripts unconditionally emitted rejection strings. One “30-parameter term/channel gain” candidate was actually an identity no-op; its “bias” used one scalar. Some profile reconstruction paths implemented only device-alpha variants. Those labels are not proof that every declared family was fitted or evaluated correctly. Preserve them as negative/limited historical attempts; any renewed estimator needs a faithful mathematical implementation, explicit fitted state, source-order/session checks, and a genuinely executed promotion rule. Do not reuse those scripts' hard-coded verdicts as acceptance.

## Training attempts and export parity

True paired input and single-image self-pair are explicit `TrainingInputMode` values in the maintained research training package. Self-pair changes observations, not native targets or cached bytes. The historical heads-only mixed experiment combined true/self-pair Huber losses and a detached true-prediction consistency target, with a tiny learning rate; it used permissive checkpoint loading and unguarded reusable outputs. Its checkpoint/report wrapper is retired. A future repeat must use provenance-checked restricted loading, fresh run directories, finite optimization, frozen splits and actual calibration-selected checkpoints.

The old card reported short self-pair candidates at normalized heldout MAE .80226 and .78751, both worse than the .78220 self-pair baseline; neither was promoted. Universal → frozen paired cascade reported .82674, worse than standalone universal .82233. The cascade used the universal model's predicted unstyled thumbnail; an oracle supplied the real native disabled thumbnail. Do not conflate those inputs or present the oracle as runtime capability. These are historical private measurements, not newly executed CI results.

The old Core ML parity harness compared key1/light-map errors at .02 tolerance, GTC byte error up to 2 and scalar discrepancy .1, against Python candidate outputs. Repeating that claim requires the exact training checkpoint, normalization/layout, source bytes, Core ML package, compute configuration and orientation-aware serialization. Those private inputs are not available from the package alone. Current `CoreMLProbe.swift` compiles and executes the public package on synthetic inputs with finite shape checks; it deliberately does not claim Python/native-renderer parity or actual Neural Engine placement.

## Promotion gate

A research result may advance only with frozen identities/splits, a complete relevant response matrix, no hidden fallback or label leakage, meaningful identity/shuffle/paired/ablated controls, declared device guards and repeatable source-conditioned resources. Product promotion additionally requires integration into the existing Rust intent/capability/publication owners, role-correct metadata, native-consumer checks and—when claimed—real Photos import/edit/save/reopen evidence. None of the preserved hypotheses is automatically a product defect or a shipped feature.

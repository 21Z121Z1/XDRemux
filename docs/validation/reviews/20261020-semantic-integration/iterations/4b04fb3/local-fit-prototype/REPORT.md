# Local key1 accumulation prototype

The smaller v2 prototype is worth retaining for an Apple ARM64 check. The subsequently completed pre-optimization native profile measures key1 fitting at 1.486920 s, so it is a material stage. On this Linux x86_64 kernel probe v2 reduced median fit time from 1.863121 s to 1.143775 s (ratio 0.613903), with every tested output byte unchanged. That Linux result is not a native/E2E result and does not establish the cause of the current Styles regression. This subtask produced the patch outside integration; applying it to the product belongs to the main thread's separate review and native checks.

## Scope and identities

Both owned detached worktrees were created at `1b2c07da2cb2b0c89c64ca546148fe5d68d75a97`:

- `baseline-source/` remains clean at that SHA.
- `candidate-source/` changes only `crates/xdremux-engine/src/apple_photographic_styles.rs`. Its current contents are the smaller v2 prototype.
- Both are worktrees of the same XDRemux repository. No other repository was operated on. No commit, push, dispatch, rebase, or integration edit was performed.

The final proposed patch is [v2-no-explicit-residual/candidate.patch](v2-no-explicit-residual/candidate.patch), SHA256 `31be7f073e7ac635492d010910121789faa33e8c69f3b01b91e226b4207aa358`. The worktrees are retained for review; the v2 directory's source entries are aliases of these same two trees. Generated public-primary requests/payloads and probe binaries remain in this external local audit directory and were not uploaded. They must not be added to public artifact uploads or repository documentation; publishable evidence is limited to code, hashes, statistics and text logs.

| Identity | SHA256 |
| --- | --- |
| Original engine source | `a13285bfb581893e592630051402ce2abd176ab4261a1b4716d674422310931b` |
| v2 engine source | `b1f58170b7bc6504eb7e1957c481bb8cd6d32e91ccfbbbca5ee0213cb615cc81` |
| Baseline probe binary, identical in two fresh builds | `53577cae4da9e2e4a536fec8c4b3776466c697c1695913e5c192fe92ddb92b9a` |
| v2 probe binary | `21791b100e7bffe657bc441e525832715a4d255e939a68a927a638857858456d` |
| Public input fixture | `32a2c5df1d3ca14e64d24ad291bc68067edcaaec6a7f9fc760d331bbd6576cbc` |
| Prepared request, both independent preparations | `24a7ba9d2661da3c006a4074ba99767bc16b7981cc4014a7202da53b18eb1b34` |
| Public-primary synthetic-target key1, baseline and both variants | `2a5bd857309865d3030a80dc4d62ef6ec4889066134f27d336268d119fe1b054` |

Each external manifest binds its engine/codec/HDR dependencies to the owned worktrees. Baseline/candidate targets are separate; v2 uses fresh `/tmp/xdremux-style-fit-accumulation-v2-target/{baseline,candidate}` targets. The v2 external lock files are byte-identical, and every dependency `(name, version, source, checksum)` is checked to exist in the pinned repository lock. Neither `RUSTFLAGS` nor `CARGO_ENCODED_RUSTFLAGS` was set.

Host/toolchain: Linux 6.18.55, x86_64, Intel Xeon Platinum 8573C, 5 exposed CPUs; rustc/cargo 1.99.1, LLVM 23.1.1, installed libheif 1.19.8 with matching pkg-config metadata. Full commands, manifests, binary/source/lock hashes, tracked status, host output and build logs are in [v2-no-explicit-residual/build-receipt.json](v2-no-explicit-residual/build-receipt.json).

## Why this change, and what was rejected

Baseline release disassembly was inspected before editing. The three `target - source` subtractions were already hoisted to the pixel level by LLVM, before the spatial/plane loops. Therefore an explicit residual array is not needed to claim removal of repeated subtraction. The baseline normal-column loop still loads `basis[row]`, multiplies it by weight inside that loop, and executes a normal-vector bounds branch for each normal entry; the RHS also retains index/bounds work.

The smaller v2 change:

1. Borrows one already-indexed cell as a 100-value normal slice and 30-value RHS slice.
2. Iterates fixed rows of 10 and 3 values respectively.
3. Computes `weighted_basis = weight * basis[row]` once per row.
4. Updates the upper normal triangle using `weighted_basis * basis[column]` and RHS using `weighted_basis * (target - source)`.

This retains the original left grouping `(weight * basis[row]) * basis[column]` and `(weight * basis[row]) * (target - source)`. For each accumulator, source iteration/update order remains `y, x, spatial y, spatial x, plane, row, column/channel`. It does not reassociate sums, factor a normal product as `weight * (basis[row] * basis[column])`, use FMA, change scalar precision, pre-sample, approximate weights, modify Gaussian plane normalization, solve fewer systems, or change the solver/ridge/clamp/half conversion. Input validation, invalid-request returns, allocation shape, and downstream failure paths remain in place. The slice bounds follow the same existing cell indexes; no unchecked access or unsafe code was added.

The v2 disassembly shows the row multiply outside the normal-column loop and no per-entry normal/RHS Vec-bound check in that inner loop. It uses packed operations on independent RHS channels; no per-accumulator reduction order is changed. The function grew from 8,427 to 8,513 machine-code bytes. [baseline-local-fit.asm](v2-no-explicit-residual/baseline-local-fit.asm) and [candidate-local-fit.asm](v2-no-explicit-residual/candidate-local-fit.asm) preserve the complete emitted function. This is evidence for the mechanism on this compiler/target, not proof of ARM64 code generation.

The first combined variant explicitly extracted a residual array. It also passed all byte comparisons and improved the median in its own window, but the residual work was already optimized by LLVM. That extra source change was discarded from the recommended variant. The first variant's complete [patch](candidate.patch), binary (retained locally; intentionally excluded from public evidence), [build receipt](build-receipt.json), [comparison receipt](comparison-report.json) and assembly remain for inspection. The v1 and v2 timings are different windows and are not an interleaved v1-v2 experiment; do not claim that removing the explicit residual array caused any cross-window timing change.

## Probe inputs and byte checks

The same harness source is compiled separately against each actual public `apple_style_revert_local_data` implementation. There is no copied local-fit implementation in the harness.

For the public case, each arm independently decodes `uhdr-portrait-01.heic` through the real portable libheif provider, linearizes sRGB and applies the actual public linear-area resize to 770×1024. The recovery target is deliberately synthetic: `v * (0.85 + 0.1 * v)`. It is **not the real HDR-derived Styles target**, does not use Apple frameworks, and does not produce a validated Styles HEIF. The two independent prepared requests are compared byte-for-byte. Timed calls then share the same immutable request file path; loading that request and writing results are outside the timer.

The comparison drives 26 requests per arm:

- 16 valid requests: the real-public-primary/synthetic-target pair; zero and signed-zero single pixels; 1×37 and 41×1 strips; small and constant rank-deficient pairs; 11×8, 12×9 and 13×10 grid boundaries; odd/even footprints; identity gradient; deterministic random 257×193 pair; negative/HDR-like samples; extreme finite samples whose plane weights underflow.
- 10 invalid requests: zero width/height; pixel/value count overflows; short source/target; source NaN/+infinity; target NaN/-infinity. Each must return the expected invalid-request error rather than succeed.

The comparator reads and directly compares full payload/error bytes. It records hashes as identifiers but uses `a == b` plus first-difference reporting for the decision. It does not accept a mismatch because a hash or metric looks close.

For each variant, 43 direct byte comparisons passed: independent preparation, 26 request results, and all 16 warmup/measured public-case payloads. All valid payloads are 51,840 bytes; all 10 invalid-request error texts match. The public-case payload also matches the pre-existing root probe's `2a5bd857...` output. The fixture hash is rechecked after measurement and remains unchanged.

## Interleaved measurements

Each variant uses one warmup per arm followed by seven paired rounds with rotating AB/BA order. Every sample is a fresh process and calls the real public producer once. The timed region is just the local-fit call; it includes the function's normal allocations and solver but excludes input decode/resize/load and output writes. Python child user/sys time covers the whole probe process and is labeled separately.

v2 result, from [comparison-report.json](v2-no-explicit-residual/comparison-report.json) and [comparison-run.log](v2-no-explicit-residual/comparison-run.log):

| Pair | Baseline kernel s | v2 kernel s | v2/baseline |
| --- | ---: | ---: | ---: |
| 0 | 1.827061 | 1.142835 | 0.625505 |
| 1 | 2.026434 | 1.198337 | 0.591352 |
| 2 | 1.833106 | 1.172990 | 0.639892 |
| 3 | 1.890863 | 1.137890 | 0.601783 |
| 4 | 1.755015 | 1.146836 | 0.653462 |
| 5 | 1.863121 | 1.129152 | 0.606054 |
| 6 | 1.881571 | 1.143775 | 0.607883 |
| Median | **1.863121** | **1.143775** | **0.613903 ratio of medians** |

Median paired ratio is 0.607883. Whole child-process user medians are 1.860653 s baseline / 1.148237 s candidate; sys medians are 0.053152 / 0.040822 s. These CPU medians do not necessarily come from the same rows as the wall medians.

v1's separate window was noisier: baseline median 2.539575 s / candidate 1.191882 s, ratio 0.469323, median paired ratio 0.558677. Six pairs favored the candidate; one pair was slower at candidate 2.385565 s / baseline 1.860369 s. That adverse pair is retained in the report. No samples were excluded and no extra run was used to replace it.

## Verification, reproduction and decision

Both variants compiled through the external Cargo manifests with `--locked --offline --release`. The final candidate passed `rustfmt --check --edition 2021` and `git diff --check`. No product tests or integration source changes were added for this prototype; functional evidence is the direct public producer calls and complete output comparisons. The independent baseline and candidate builds have distinct targets, manifests bound to the respective exact source tree, and logged command/environment identities.

The local scripts are `build.py`, `compare.py` and their v2 copies; the exact shared Rust harness is `probe-main.rs`. A fresh reconstruction should create new owned detached trees at the pinned SHA, apply the chosen preserved patch only to the candidate tree, bind the external manifests to those trees, use fresh target directories, and execute the recorded Cargo/probe commands. The current retained candidate tree is v2: the top-level v1 receipt names its original captured source hash/patch, not today's candidate contents. Do not rebuild v1 against the current v2 tree and describe it as a v1 reproduction.

Retain **v2** as a narrowly scoped optimization candidate. Reject explicit residual extraction as unnecessary source work because LLVM already performs that CSE. Do not introduce subsampling, weaker validation, fake metadata, fewer Texture roles/effects, a precision downgrade or a changed budget to obtain speed.

## Pre-optimization native profile, read after the prototype

The existing [1b2 native job](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421273/job/118909654219) completed the extra diagnostic conversion and real validation. Its [raw text log](native-profile-1b2-job118909654219.log) and [parsed final summary](native-profile-1b2-summary.json) are retained. These measurements precede this accumulation patch and are not native evidence for v2.

The extra candidate-only diagnostic wall is 15.346095 s, user 14.36 s, sys 1.87 s, with `status=validated`. Stages include base conversion 3.327722 s; semantic production 4.027638 s; scene derivation 1.340215 s; key1 fitting 1.486920 s; Texture generation 3.926049 s. It confirms key1 is worth measuring and optimizing, while semantic/Texture/base remain larger costs. Even hypothetically removing the full key1 time in this particular diagnostic window leaves about 13.859 s, above the unchanged 11.732 s absolute median limit; this is only a same-window bound, not a forecast or a reason to change the limit.

The formal gate in that job still fails Styles median 17.221 s / p95 17.955 s against 11.732 / 15.406 s. Its separate same-build-scope main/candidate interleaved medians are 12.742538 / 15.380543 s, ratio 1.207024, user medians 11.6 / 13.9 s. Do not mix those interleaved medians, the earlier formal window, and the one diagnostic conversion into paired stage deltas.

Remaining evidence for v2 is an ARM64 baseline/candidate byte comparison and a new exact-head native/E2E measurement. A minimal additional behavior check is a one-time same-runner, two-binary comparison at the existing key1 consumer boundary: compile baseline 1b2 and the proposed head against the same small probe, feed both one actual Styles producer request plus the deterministic/invalid boundary cases, compare full output/error bytes, and emit only hashes, difference location and timing. The actual RGB request and binaries remain temporary runner-local data and must be discarded/excluded from uploads. This does not require a permanent old-solver implementation, a generic multi-arm owner, or a weakened numerical assertion. Existing engine/native tests and the unchanged full performance gate remain required; the temporary probe cannot replace them.

The current full-resolution/local-fit producer existed in f9's successful run as well as the later failures; improving its Linux kernel does not establish why those native run windows diverged. This report does not declare the absolute budget failure resolved or claim ARM64 equivalence before those checks execute.

## Publication adaptation

This public copy changes only the excluded v1 binary link and this notice. Original REPORT.md SHA256: `dea74a2bedca4e9b4597a41b9b3d48afac057dc257cfc8be4c9fa8824e7b7db8`. Generated media, RGB request buffers, key1 payloads, probe binaries, Cargo targets and source worktrees are intentionally absent. Captured receipts retain original execution paths as identities, not public download links. Root independently reviewed and applied the v2 engine bytes in 4b04fb3; this prototype report keeps its original pre-application scope.

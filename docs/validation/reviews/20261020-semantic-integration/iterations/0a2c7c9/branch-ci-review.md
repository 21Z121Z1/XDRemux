# XDRemux residual-branch and CI review

Review date: 2026-10-20. Repository: `21Z121Z1/XDRemux`. Main comparison target: `89d806c8aec961a7991eb3e6d6e2b1370aed8791` (`Generate inferred Texture Styles with Photographic Styles (#70)`). The parent confirmed through the repository connector that this repository is public. This review inspects only XDRemux source, Git objects, and the parent's existing audit records. References to another repository in existing workflow YAML were inspected as configuration; that repository's contents were not inspected.

This is an external, current audit record, not stable product documentation. It intentionally records branch identities, integration state, and outstanding acceptance work outside the product repository.

## Scope and authority

Read `AGENTS.md`, `docs/development.en.md`, `docs/quality/testing.en.md`, `docs/apple-features.en.md`, `docs/validation/README.en.md`, and the user-selected `/workspace/xdremux-audit/dsh-find-simplifications.SKILL.md`. Followed the skill's independent-diff, producer/consumer, rejected-alternative, and retained-obligation guidance. DeepSeek-specific package assumptions do not describe XDRemux.

Initial audit was read-only. The parent subsequently authorized exactly two writes in this domain: this external report and the obsolete host-OS summary sentence in `/workspace/XDRemux-integration/.github/workflows/performance.yml`. No other repository file was edited by this reviewer. No index staging, commits, remote writes, branch deletion, workflow dispatch, billing change, or protection change was performed.

## How branch coverage was established

Used `git rev-list --left-right --count`, `git merge-base`, `git diff origin/main...BRANCH`, endpoint diffs, `git cherry -v`, exact object/file comparisons, `git grep`, and the canonical producer/consumer paths. Three-dot diffs isolate a branch's contribution at its older base; endpoint diffs additionally reveal newer main changes that would be lost by wholesale replacement.

`git cherry` reports all 235 texture-style commits as `+` against main. That does not prove 235 missing capabilities: commit-level patch IDs do not recognize combined, reworked, or deliberately retired behavior. The parent identified #70 as a squash. Coverage below is therefore based on the resulting behavior and retained evidence, not only ancestry or commit count.

| Residual branch | Exact tip | main-only / branch-only commits | Decision |
| --- | --- | ---: | --- |
| `origin/feat/rust-cli-distribution-i18n-20260924` | `c6d307af662ed516f6e3994c039513359cb98610` | 61 / 6 | Real unique CLI presentation and distribution work remains. Preserve backup and require explicit parent disposition; do not silently classify it as covered. |
| `origin/research/texture-style` | `f8fcf2067c506e87665e9ea113324db1a57d4cf7` | 117 / 235 | Old generation is retired or superseded; durable contracts are already retained in main. Do not restore its product hook. |
| `origin/research/texture-native-20261019` | `18340989590d451f5676924feb03c70baa503e1a` | 117 / 239 | Four additional commits add opaque transport and research evidence. No missing public CLI behavior was established. Preserve evidence and branch backup. |

The parent's `backup-manifest.json` records durable `backup/semantic-review-20261020/...` refs for all three branches, remote verification, a fresh bare-repository restoration, exact commit/tree comparison, and `git fsck --full --no-reflogs`. This reviewer read that manifest; it did not independently create or fetch those backups.

## Distribution and CLI i18n: unique behavior requires a decision

### Problem

The branch contains two actual user-facing additions absent from the reviewed main: bilingual human terminal presentation and a self-contained macOS binary package. Excluding the branch wholesale would leave those capabilities unintegrated. Conversely, restoring its entire older tree would remove current Cinematic, inferred Texture, SmartStyle/final-byte validation, and other newer main changes.

### Producer and consumer evidence

The six commits are `8be9f55`, `a97ba55`, `055143c`, `5f350a5`, `3bfc2c0`, and `c6d307a`.

* `crates/xdremux-cli/src/i18n.rs` defines `--lang`/environment selection and translates clap descriptions before rendering, rather than replacing arbitrary output substrings. Precedence is explicit `--lang`, `XDREMUX_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG`, English. Empty environment values are ignored; unsupported environment locales fall back to English. Unsupported explicit language remains a parser error. Non-Unicode paths and arguments after `--` remain opaque.
* Branch `crates/xdremux-cli/src/lib.rs`, `categorize.rs`, and `validate.rs` route the selected language to human output. Machine JSON, option/subcommand names, enums, schema, and exit codes remain invariant by design. Clap details and lower-level diagnostics can remain English; this is not complete translation of every diagnostic.
* Branch `crates/xdremux-cli/tests/i18n.rs` contains process-level help, precedence, error routing/exit code, path preservation, JSON byte-invariance, and real-fixture conversion/validation/categorization assertions. Tests were read, not executed on this branch in this review.
* `scripts/distribution/build_native.py` downloads checksum-pinned libde265/x265/libwebp/libheif sources, builds the native architecture, disables libheif runtime plugins, and preserves the intended HEVC/SharpYUV closure.
* `scripts/distribution/macos.py` traverses Mach-O dependencies, restricts them to the pinned prefix/system frameworks, copies the closure, rewrites install names to loader-relative paths, ad-hoc signs, audits architecture/rpaths/signatures, copies source archives/license notices, and emits a normalized tar archive plus SHA-256.
* `.github/workflows/macos-distribution.yml` builds `arm64` on `macos-26` and `x86_64` on `macos-26-intel`, using the exact requested source SHA, then stages/audits/archives/uploads artifacts. These are standard runner labels; no paid runner is needed.

The current main already discovers an adapter at `../libexec/xdremux-apple-adapter`: `crates/xdremux-cli/src/lib.rs:339-359`. That part is covered, rather than an additional missing runtime behavior.

### Acceptance gaps and rejected wholesale import

The final distribution workflow audits linkage and signing but does not unpack the emitted archive, run the packaged executable from a new directory with build/environment dependencies absent, perform an actual fixture conversion, or validate the packaged output. A package audit is not proof of functional product distribution. No native packaging execution was performed here.

Its package README advertises `--apple-styles` on macOS 26. #70's current Styles switch also requires macOS 27 private Texture APIs, so that example and `validation_target_os` would need fresh qualification. The macOS 15 deployment target does not establish that every feature works on macOS 15/26.

The branch moves `Package.swift` and the adapter to `platforms/apple/`, but its manifest contains only the older Swift adapter target. Main `Package.swift:21-55` now also owns `XDCinematicNative` and `XDTextureNative`. Copying the old manifest or moving only the old source set would drop those current obligations. A package location change is an ownership choice, not a prerequisite for the unique bilingual feature.

The translation table predates main's Cinematic arguments and feature-specific `validate --apple-styles` behavior. A selective i18n port must update descriptions and keep main's parser/runtime calls. The branch does not update the canonical `docs/cli.en.md` and `docs/cli.md` for the new language contract; those documents would need updates if the feature is promoted.

### Proposal and parent disposition

Retain the verified branch backup. The parent must explicitly choose to selectively port the two unique capabilities or to defer/exclude them with the above acceptance gaps recorded. This report does not authorize silently treating them as already merged, deleting their branch, or claiming they have passed product distribution acceptance.

If porting, keep main's current product and native target graph. Validate both languages across current help/output paths and JSON contracts; validate a relocated unpacked package with representative conversion and output validation. Reuse the existing distribution tooling where coherent rather than creating a second installer/runtime policy. Production release signing/notarization and distribution licensing are separate from an ad-hoc CI artifact; no publication was performed here.

Strongest reason to retain: these are concrete user capabilities with explicit consumer tests and a complete packaging implementation. Strongest reason to defer promotion: it is old-base work without demonstrated current packaged conversion, with current native-target and OS-support differences still to reconcile.

## Texture style branch: superseded generation and retained knowledge

### Current and historical paths

The old branch adds `crates/xdremux-heif/src/texture_styles.rs`, a runtime augmentation hook, a diagnostic v16 Styles plist writer, and reusable research probes/contracts. Its generator copies one producer-backed 2019 skin image/descriptor into twelve differently named 2026 semantic roles, writes a fixed 216-byte Texture plist and 133-byte tag-84 plist, and can synthesize an Apple MakerNote/source-hash UUID-shaped PhotoIdentifier. These source/role claims do not become true because a parser accepts the container.

Main intentionally retired those shortcuts. Ancestor `902d7f2` reconstructs bounded ABI-checked probes and fixes sparse-oracle materialization; `95ec68d` preserves seven original contracts with commit/blob/SHA-256 identities. The current owner is `research/texture_styles/`, especially `README.en.md`, `PersonInputProbe.m`, `InputTrace.m`, `RuntimeInventory.m`, `sparse.py`, `oracle.py`, and `evidence/historical-contracts.json`.

An exact retained-content check confirms branch `scripts/research/texture_style_native_standard_contract.json` and main `research/texture_styles/evidence/native-standard-matrix.json` both have SHA-256 `2bb45a9273a270802da6148b21431666a03dd8d0296915532e396bf197277d4d`. Thus moving back the old JSON or its multiple competing patchers is not necessary to preserve those hypotheses.

#70 supplies a materially different product effect path:

1. `crates/xdremux-runtime/src/apple_styles.rs:344-348` stages the assembled Styles bytes and calls `apple_texture::generate`.
2. `crates/xdremux-runtime/src/apple_texture.rs:101` binds source and upright analysis-raster hashes, checks completed native observations, distinct person/instance mappings, and mask codec controls.
3. It requests original Mattify, UnderEyeBrighten, and SkinSmoothStandalone operations and checks completion, native input consistency, geometry, and returned statistics. Empty optional groups remain omitted.
4. `crates/xdremux-heif/src/texture.rs:103` assembles the newly inferred resources and checks original item payload preservation. Existing Texture metadata is rejected rather than ambiguously appended.
5. Final native readback and MakerApple field checks run before `AtomicFilePublisher` publication. `scripts/apple/check_texture_styles.py` drives convert, batch, app switch, feature-aware validation, unchanged media payloads, and failure-preserves-output assertions.

This is semantic supersession, not equality with the old generator. Current source-backed derived context still does not prove complete camera FSINC, device rendering, or Photos edit/save/reopen behavior. This review did not rerun those native checks.

### Recommendation, alternatives, and risk

Exclude the old automatic generation hook, fixed capture templates, aliases, unsafe/raw-memory acquisition probes, and temporary workflow machinery from the integration. Keep the already promoted bounded probes and retained contract evidence under their current owners. Preserve the original branch/backup for provenance.

Retaining all old code as another selectable backend would restore a second product path and untrue resource claims. Treating an old device-admission report as full role truth or interactive editability also fails the current evidence contract. The strongest reason to retain historical artifacts is reproducibility of rejected hypotheses; that purpose is served by exact retained contracts and the backup, without reviving product APIs.

## Texture native branch: four extra commits and evidence limits

`origin/research/texture-style..origin/research/texture-native-20261019` contains exactly `8231867`, `0a46213`, `cf6da4c`, and `1834098`.

* `8231867` removes unsupported generation and introduces `inspect_texture_style_heif`/`preserve_texture_style_heif`. The inspector checks the exact Texture URI, local item locations, relationships, and bounds; preservation returns all original bytes. The compatibility `augment_texture_style_heif` entry point calls preservation and errors when Texture data is absent. It does not produce a new edit.
* `git grep` finds those APIs only in the public re-export, `examples/texture_transport.rs`, `tests/texture_transport.rs`, and `scripts/research/verify_texture_transport.py`. There is no CLI/runtime consumer. The diagnostic `apple_texture_style_property_list` is exported but no longer used by runtime on that branch.
* `0a46213` restores the consumer regression script's executable mode; it is branch-relative maintenance, not a new user capability to restore over the newer script.
* `cf6da4c` and `1834098` add native logs, hash-bound JSON summaries, readers/validators, and narrowly scoped Git attributes preserving log bytes.

The branch's `claims.json` explicitly separates metadata controls from whole-image eligibility: 21 complete logs, 147 payload hashes checked, twelve ImageIO codec cases, zero successful Bloom renders, four failed ordinary Styles conversions before final assembly, no device editability, and unverified ZIP artifact bytes. These are retained observations, not fresh executions by this reviewer and not evidence of successful general Texture generation.

The branch changes ordinary `--apple-styles` back to v15 without Texture augmentation. Importing that change over #70 would remove its newly implemented same-switch Texture behavior. The opaque byte-preserve API has value for a future explicit transport consumer, but introducing it now as another product contract has no demonstrated shipped caller and does not fix the old missing-producer problem better than #70's current transaction.

Recommendation: preserve the branch backup and its extra native evidence; do not import the behavior regression, unused product API, old-base source tree, or session-specific `NATIVE_TODO.md` as canonical product knowledge. Archive/consolidate durable negative results only if a current research owner needs them. Strongest retention case is exact native failure/provenance evidence. No additional mandatory public behavior was established by this branch.

## Runner, cost, and capability review

At the reviewed main, six workflows expand to twelve jobs:

| Workflow | Job runner(s) |
| --- | --- |
| `rust-cli-core.yml` | `ubuntu-latest`, `macos-latest`, `windows-latest` |
| `rust-proxdr-real-fixtures.yml` | `ubuntu-latest` |
| `ci.yml` | `xcode-27` |
| `completion-gate.yml` | `xcode-27` |
| `performance.yml` | `xcode-27` |
| `research.yml` | one `ubuntu-latest`, three `macos-26`, one `xcode-27` |

All declare `contents: read`. No larger/large/xlarge, self-hosted, custom runner group, or paid label is present. The subsequently inspected PR68/69 workflows also use only `xcode-27` and `macos-26`.

The premise that `xcode-27` is nonstandard or paid is incorrect as of this review. GitHub's [standard runner table](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) explicitly lists it in the same arm64 standard-runner row as `macos-26` and states that standard runners are free and unlimited for public repositories. GitHub's [official preview announcement](https://github.com/actions/runner-images/issues/14404) identifies the label and notes that its host OS became macOS 27 on 2026-09-16. Preview status and paid/larger-runner status are different facts. Parent connector evidence confirms XDRemux is public. No billing settings need to change.

Keep `xcode-27` for the native macOS 27 contract. Merely choosing `macos-latest`/`macos-26` does not supply macOS 27 APIs. `Sources/XDTextureNative/infer.m`, `statistics.m`, and `readback.m` dynamically use Vision/CMImaging/NeutrinoCore/PhotoImaging classes and reject unavailable operations. `docs/apple-features.en.md` explicitly scopes the current Texture operation to tested macOS 27 APIs. The exact-head Texture integration, product Texture/app switch test, joint Styles performance workload, and Styles version probe cannot all be relabeled as passed on macOS 26 without executing the required behavior. Do not introduce skip/continue-on-error waivers to make such a downgrade green.

`tests/validation/test_workflow_configuration.py` does not hard-code `xcode-27`; its portability assertion intentionally fixes the existing `[ubuntu-latest, macos-latest, windows-latest]` matrix. The performance environment record explicitly writes `runs_on=xcode-27` and already records `sw_vers`, architecture, Xcode, and Swift. The committed baseline is `benchmarks/macos27-arm64-product-baseline.json`; changing the runner or loosening its thresholds is not needed to fix presentation.

### Authorized correction completed

Only `/workspace/XDRemux-integration/.github/workflows/performance.yml:164` was changed:

```bash
echo "Runner: GitHub Actions hosted $(sw_vers -productName) $(sw_vers -productVersion) ($(uname -m)), label xcode-27. These are not local-device measurements."
```

It replaces the stale literal `macOS 26 (arm64)` with observed OS/version/architecture while retaining the real label and existing environment artifact. Runner choice, timeouts, performance budgets/thresholds, permissions, triggers, protection, tests, and artifact settings were not changed by this reviewer. The change remains unstaged and uncommitted for the parent to integrate.

## PR68/69 additional workflow ownership review

Reviewed local integration files and the existing PR snapshot, not new remote execution. PR68 tip is `1f29e3cdc1b8fd49d56718ceecf96b8de80ec817`; PR69 tip is `7952d0b19c6b47738cc0722acae57465a243186f`. The integration was in an active merge with unrelated conflicts when inspected; conclusions concern the files present at that point.

### Problem and retained obligations

Canonical `docs/quality/testing.en.md:68-73` says one reusable `research.yml` owns portable research tests and native carrier readback. Main's minimal-workflow test expects six workflows. PR68 adds `style-tone-sweep.yml` and `system-heif-hdr-api.yml`, adds explicit prose exceptions, and adds them to the expected set. Those exceptions intentionally retain measurements, but maintain two extra triggers, checkout/build/dependency sequences, concurrency groups, and artifact owners beside the canonical research workflow. They are genuine measurement workflows, not ref-writing trigger hacks; their unique evidence should remain.

PR69 adds `orientation-imageio.yml`, a product orientation/native readback regression, without an expected-set exception. Its checkout has neither an explicit head SHA nor `persist-credentials: false`, so on a pull request it normally tests GitHub's merge ref rather than the canonical exact PR head. It also lacks the shared concurrency policy. Its runner is nevertheless a standard free runner, and its actual test/readback commands are useful.

### Proposed consolidation into existing owners

Do not delete measurement behavior merely to restore a file count. Move the jobs/steps to existing owners, retain the actual scripts, then remove redundant workflow wrappers and their documentation/test exceptions only when the consolidated jobs cover the same trigger/evidence scope.

1. **PR68 tone response → `research.yml` job `style-tone-response`, `xcode-27`.** Keep exact-head checkout with credentials disabled; keep the pinned consumer checkout at `37dd551a949047ed14580efa0c0a2556922791b5`; install libheif and the isolated NumPy/Pillow venv; build the current Rust CLI and complete Apple adapter; run `scripts/apple/run_style_tone_sweep.sh` with its existing `RESEARCH_ROOT`, `TONE_OUT`, `XDREMUX_CLI`, and adapter values; upload tone evidence on success/failure with one-day retention. The script's native tone-zero sanity check and nonzero failure must remain authoritative. Do not claim a tone score is Photos edit/save/reopen acceptance.
2. **PR68 system HEIF HDR → `research.yml` job `system-heif-hdr`, `xcode-27`.** Retain SDK surface probe `scripts/apple/probe_system_heif_hdr_api.sh`; build/setup the product and adapter; run the synthetic native `cargo test --locked -p xdremux-runtime --test system_heif_hdr -- --nocapture` with `XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE`; retain the same pinned consumer checkout and `scripts/apple/measure_system_heif_hdr.sh`; upload both SDK and measurement evidence with one-day retention. A failed experimental convert remains a failed job with available diagnostics, rather than a successful system-gain-map/Styles claim. The PR snapshot itself reports a failure before Styles publication and no system-encoder tone score; that recorded result was not rerun here.
3. **PR69 orientation → existing product integration, preferred.** This test verifies a current Rust writer/codec regression, so keeping it in `ci.yml`'s native product execution is semantically cleaner than making a research result stand in for product acceptance. After libheif/Rust setup, run `XDREMUX_ORIENTATION_OUTPUT=... cargo test --locked -p xdremux-codec --test orientation_roundtrip -- --nocapture`, then `swift scripts/apple/read_oriented_heif_imageio.swift .../oriented-gain-map.heic .../expected.json | tee .../imageio.json`, and retain the one-day failure diagnostics. Reuse the already exact-head checkout and existing upload or add the narrowly scoped artifact upload.
4. **PR69 alternative if one research carrier-readback owner is chosen.** Add the same two orientation steps as a `research.yml` `macos-26` job with explicit `${{ github.event.pull_request.head.sha || github.sha }}` checkout, credentials disabled, and the existing workflow concurrency. Keep product regression evidence separately in the product gate; do not reinterpret this research job as `exact-head`.

For `research.yml`, preserve main-only push, PR, manual no-input execution, `contents: read`, shared concurrency, and distinct job failure states. Its existing path filters do not cover crate-only edits. Add the source path unions needed by migrated jobs: current engine/runtime/hdr/cli/Apple adapter and `fixtures/proxdr/**` for the PR68 jobs; HEIF/codec/format paths and the ImageIO reader for orientation if it moves there. `scripts/**` and workflow changes are already covered. Independent jobs can retain their OS needs; merging files does not mean running macOS 27 operations on macOS 26 or combining failures into one permissive script. There is no need to introduce a new paid runner, mutate a budget, or broaden credentials.

### Alternatives and acceptance criteria

Keeping PR68's explicit exceptions is defensible if the parent deliberately revises the single-owner policy: these are durable measurements with different durations and a pinned external consumer dependency. It still incurs separate trigger/build/concurrency ownership and leaves the introductory single-owner statement misleading. This report recommends consolidation for net maintenance reduction, while preserving all observable measurements.

For orientation, keeping a standalone product workflow would require a documented product-owner reason, exact-head/credentials/concurrency fixes, and a deliberate minimal-set test change. File count alone is not sufficient reason to discard it, and blindly adding an expected-set exception does not resolve its source-identity difference.

Before considering consolidation complete, validate workflow syntax, the minimal-set and exact-head/read-only policies, correct path coverage, and the native jobs on their applicable hosts. Native errors must fail the job even if failure artifacts upload. Current source/SDK/native/device limits must remain visible. No workflow consolidation was implemented by this reviewer.

## Checks, limitations, and handoff

* Initial main: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.validation.test_workflow_configuration` passed all 13 tests.
* Integration after PR68/69 files appeared: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.validation.test_workflow_configuration.WorkflowConfigurationTests.test_workflow_set_stays_minimal` failed with the concrete extra file `orientation-imageio.yml`. PR68's modified test already permits its other two new wrappers. This is an integration finding, not a failure caused by the one-line performance-summary edit. The test was not modified by this reviewer.
* `git diff --check -- .github/workflows/performance.yml` passed after the summary correction. Reviewed the exact diff: one line, with original environment collection intact. No macOS shell/runtime execution of `sw_vers` was possible on this Linux host.
* `git grep` consumer audit, exact retained JSON hashes, Git ancestry/contribution commands, workflow/source reads, and backup-manifest inspection were completed as described. No missing native/device evidence was represented as passed.
* A prior harmless CLI query, `gh repo view 21Z121Z1/XDRemux --json visibility,nameWithOwner`, was rejected by the environment's GraphQL access with `Forbidden`; no side effect occurred, and the action was not retried. The parent subsequently supplied successful connector evidence that the repository is public, resolving the visibility question for this report. This was not an automatic approval-review rejection.
* Still for the parent: decide explicit distribution/i18n promotion or documented deferral, reconcile PR68/69 workflow ownership without dropping evidence, finish the active merge, run the appropriate product/native checks, and bind any final completion receipt to the resulting committed HEAD. This reviewer made no commit and cannot claim exact-HEAD completion for the parent's unfinished integration.

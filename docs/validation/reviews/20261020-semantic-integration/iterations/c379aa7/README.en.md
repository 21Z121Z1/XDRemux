# Direct Gain Map validation, product scope and Tone evidence boundary

This appendix captures source `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5`, tree
`f618057269722e87be12636b46f5292c5e0d3baf`. It is an intermediate observation
head, not final acceptance. Merge and deletion remain **NOT APPROVED**. No
protection, security, billing, release or upstream setting was changed.

## Changes and discriminating evidence

| Demonstrated problem | Smallest accepted repair | Actual check and remaining boundary |
| --- | --- | --- |
| Native System HEIF failed on a legitimate direct `hvc1` Gain Map after the previous alpha boundary passed. The portable validator and Styles appender assumed GRID. | `eca4bf968eb3a033a435ae3f007781db11d9d1aa`: validate ordered tmap roles for GRID or direct HVC1; share the complete validator with Styles assembly. Preserve actual resource IDs, color properties, payload and associations. No synthetic GRID or re-encode. | Three RED groups; 57 unit + 2 fail-closed tests, clippy/fmt pass. Reject missing/duplicate codec and precision properties, derived coded items, primary aliases and invalid storage. Actual new-head System HEIF remains pending. |
| The user selected complete arm64/macOS27 product delivery, while old settings and the added Intel package still implied broader support. | `2ea21ed6e0df09a2641a650e1407cfab657ad8e4`: align Package/App/native host guard, pinned dependency build, audited archive and docs. Keep both current native targets, full dependency closure and outside-checkout smoke. | Real RED configuration/host negatives; 46 packaging/workflow/doc checks passed. The three portable Rust CI host blocks and pinned native source manifest are unchanged. Mock Mach-O observations are not native packaging evidence. Intel results on old4b are historical; new arm64/macOS27 packaging still needs CI. |
| The old Tone job was green although all3 product extractions returned1 and all product native comparisons were missing. | `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5`: require complete process/render/numeric evidence, preserve the fixed consumer's first error and exact log hashes, export only typed facts and filtered text diagnostics. | Against the immutable4b owner, 4 existing-behavior methods produce6 assertion failures without setup errors. Current 15 focused +19 workflow tests pass. Real consumer tools were not run on Linux. SPI integer statuses are observed without guessing their undocumented success domain. Missing data cannot inherit Apple's trusted flag. |
| Native probe logs print a multiline facts dictionary; uploading whole logs could export raw dictionaries. | Keep pre-dictionary stderr and small typed exceptions; omit the known one-line extractor inventory and native final facts dictionary. | Synthetic secret-marker negatives prove the exported diagnostics exclude raw dictionaries and retain the first fault. This does not assert that old broad artifacts never contained public-fixture derivatives. |

The [author review](c379aa7-author-review.json), [HEIF review](direct-gain-map-structure-review.txt),
[distribution review](arm64-macos27-distribution-review.md), and
[Tone review](tone-response-fail-closed-review.md) identify source scopes and
actual limits. Measurement completeness does not establish correct Photos
Tone/Color editing, save/reopen or device behavior. The original4b extractor's
internal first error was not logged; this commit instruments the next native
run rather than pretending that failure is already fixed.

## Committed-head portable receipt

All7 checks in [the receipt](c379aa7-portable-receipt.json) passed on a clean
committed head; positional `agent_completion_gate.py verify` exited0 before
PVT implementation began. Receipt SHA256:
`954db2154d21e7c724b3123bc8729e80e9630a4c3c4e5b54a42b01bf44ba177a`.
The reviewed [plan](native-observation-portable-plan.json) covers locked
metadata, fmt, workspace all-target clippy, engine/HEIF/runtime all-target
regressions, all272 Python tests, current CLI build and actual Samsung
conversion plus independent pair-member validation. Linux does not prove
Swift compilation, native System HEIF/Tone/App, GPU, performance or Photos.
The earlier65-test combined review predates the last Tone privacy/status
corrections; the final7-check receipt and34-test Tone log cover those bytes.

Automatic new-head CI runs: [product/App39514720759](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720759),
[exact-head39514720814](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720814),
[performance39514720891](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720891),
[research39514720871](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720871),
[portability/arm64 package39514720882](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720882).
They were queued/running at capture, not counted as passed. No duplicate
dispatch was made. Any subsequent source head needs its own acceptance.

## Persistent recovery and proposed convergence

The source was backed up **before** the ordinary source-branch fast-forward:
`refs/tags/backup/semantic-review-20261020/integration/c379aa7`.
Its [restore log](integration-restore-c379aa7.log) proves independent remote
fetch, exact commit/tree and full fsck0. The [23-ref manifest](all-23-backup-manifest.json)
and [fresh restore log](all-23-backup-restore.log) cover all9 current remote
branch heads. No source or research branch was deleted. Previous v8 evidence
itself was [remotely restored](evidence-v8-restore.log).

The parent's selected convergence remains ordinary merge of#71 once after
final functional gates and final recovery approval. The four original PR
tips remain ancestors; their original CI is not relabeled green. Observe
actual indirect merged states, then execute only the approved exact-ref
cleanup plan. The old4b disabled operation manifests are historical, not an
execution token for this head or future PVT work. Final-head backup, graph,
hooks/config/permission checks and parent approval must be refreshed.

## Optional PVT: evidence, not implementation in this head

[Read-only research](pvt-output-research.public.md), [history inventory](pvt-history-scan.json)
and the [seven-group Linux prototype](pvt-prototype-manifest.json) are retained.
Current and all22 previously restored backups contained no PVT implementation.
The independently located [makelive revision40abfa4](https://github.com/RhetTbull/makelive/blob/40abfa4a5bc458609ddb9087d3a2743a826103a4/makelive/makelive.py#L315)
uses a true `.pvt` directory with still, MOV and `metadata.plist`; the version
key is string `"1"`. Its source/test blob hashes were checked, not executed.
Its overwriting copy strategy is excluded from the design. No third-party
program or bootstrap was installed, and its full downloaded source is not
republished here.

The one-shot prototype proves byte-identical wrapping/unwrapping of a real
validated pair and Linux exclusive directory publication against existing
files/dirs/symlinks and competing writers. Its existing CLI binary is explicitly
not claimed as an exact-HEAD build. The current CLI still rejects a package
directory. PVT product implementation is now separately in progress, with
`--live-photo-output pair|pvt`, default pair, consistent CLI/App choice and no
extra re-encoding. An inaccessible old Snoopy sample, historical user import
feedback, a structural prototype, native Photos import and dynamic wallpaper
are distinct evidence. No current native import/wallpaper pass is claimed.

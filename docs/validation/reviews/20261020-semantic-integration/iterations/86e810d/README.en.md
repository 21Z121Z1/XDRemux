# Real service facts instead of redundant help prerequisites

Candidate **86e810dfb52cbe2516d816bc0ac11b94de84d0e7**, tree
**0997860e6554abedc9999d5ea63ca7936df2e144**. The production change deletes
two redundant simctl help calls. One external-call policy regression prevents
those metadata requests from gating actual installation-service observations.
Product Rust/Swift/App, Cargo and workflow bytes are unchanged from392/af223.

## First actual failure and correction

The [392 PVT receipt](392009a-pvt-native-receipt.json),
[observation](392009a-pvt-native-observation.json) and
[raw log](392009a-pvt-job119054825290.log) show a new, author-introduced problem:
the mandatory `simctl help install` request exceeded its15-second ceiling before
the real installation coordinator was called. No real inventory, installation,
Photos authorization/baseline or three-path package sharing was reached.
Source/package hashes stayed unchanged and own simulator cleanup succeeded.
This was not an Apple media rejection or a test of the new reconciliation logic.

Printing usage is not evidence that an installation service is ready. Both
install/listapps help prerequisites are removed, rather than lengthening their
timeouts or rerunning the failed head. The actual bounded inventory replies
now supply the readiness facts. The single120s install, registration and
owned app/data/bundle/executable checks, late-result reconciliation and
failure diagnostics remain unchanged. No uncertain install is repeated.

The [structural regression](pvt-install-help-before-fix.log) fails for the actual
old mandatory external call and passes after removal.46 focused checks pass.
It is an external-call policy check, not native installation evidence. The
[clean scoped current-head gate](86e810d-portable-receipt.json) passes4/4:
all308 Python regressions, format, current CLI build and real pair/PVT
roundtrip/no-clobber. Unchanged full Rust/clippy suites were not repeated.
Actual inventory/installation, whole-PVT sharing and independent Live asset
readback still require the new head's native CI.

## Other392 native checks allowed to finish

The already-running392 performance, App and Tone jobs completed before the
source branch advanced. Their original logs are kept, not canceled/replaced
with an old or favorable result. The [performance observation](392009a-performance-observation.json)
includes the actual native release bit-equivalence test and unchanged budgets:
Styles median9.196s/p959.472s/RSS1084.078MiB pass11.732s/15.406s/1361.367MiB.

Its rotated same-runner comparison remains a material unresolved cost:
main7.920269s/candidate10.543739s, **+33.1235%**, user7.89/9.58s and
sys0.53/1.04s. This is worse than the preceding af223+17.8593% observation,
despite unchanged product bytes. It is not dismissed as host noise or hidden
by the absolute pass. No392 performance rerun or threshold change occurred.
Prior failed attempts and source-to-work/prototype evidence remain in the
inherited appendices; no complete ARM64/E2E speedup claim is made.

[Product/App receipt](392009a-app-observation.json) and
[Tone receipt](392009a-tone-observation.json) retain the exact executed cases,
render rows and source identity. These are earlier-head proofs. They do not
become86 native acceptance merely because product files are byte-identical.
Original media, source-derived native Styles, whole package representation,
exact save callback, one added Live asset/original resources/IDs/cover/hash and
owned cleanup assertions are unchanged. There is no decomposed PhotoKit pair
creation fallback or physical-device/wallpaper acceptance.

## Recovery and execution boundary

The [37-ref manifest](all-37-backup-manifest.json) and
[complete restoration log](all-37-backup-restore.log) bind every exact remote
commit/tree, non-shallow full strict fsck0, and all9 current head recoveries.
The current tag was independently restored before ordinary source fast-forward;
the other8 heads stayed unchanged and main remains89d806c8.

[Convergence remains disabled](86e810d-convergence-plan.json): final native
and exact-head gates plus parent execution review precede ordinary merge71,
actual indirect merged-state/auto-deletion inspection, persistent restoration
of resulting main and fresh exact cleanup-set/guard review. No merge/delete,
force push, release, upstream write, paid/billing, protection or global security
change occurred. Only named text evidence leaves the native runner.

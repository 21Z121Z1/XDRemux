# Owned simulator installation: command result and actual state

Candidate **392009ad9d6e1974ff154fe1705536da3c6265c3**, tree
**df25fbb2d193d9828ae1d6a4ec7a3a9d831fe1b9**. Three changed files since
af223 repair installation setup, add behavioral regressions and update the
existing verification recipe. Product Rust/Swift/App, Cargo and workflow bytes
are unchanged. This is a narrow harness repair, not another product refactor.

## Preserved first error

The [af223 native receipt](af223f0-pvt-native-receipt.json),
[observation](af223f0-pvt-native-observation.json) and
[full log](af223f0-pvt-job119039642433.log) establish:

| Operation | Actual result |
| --- | --- |
| Cold SDK canary | 5.051s, exit0. |
| Actual reader compile | 1.162s, exit0; signing also exit0. |
| Simulator boot / bootstatus | 16.600s /73.224s, exit0; reported already booted. |
| Install owned PVTReadback.app | 60s ceiling,63.052s observed, client timeout, empty stdout/stderr, actual installed state unknown. |
| Photos authorization, baseline, three whole-package UI paths | Not reached. |
| Original synthetic source/package | Hashes unchanged; own simulator shutdown/delete succeeded. |

There was no Apple rejection error code. Booted state does not prove the
installation service completed a request, and a client timeout does not prove
the app is absent. The old device was deleted without a bundle/container
readback, so its actual old installation outcome cannot now be reconstructed.
The parent's independent artifact12321573686 checksum is attributed explicitly;
this executor parsed the original console receipt and does not claim ZIP access.

## New single-install boundary

The existing probe now reads the owned simulator's installed-app inventory with
bounded service-readiness polls. JSON/XML/binary plist replies are parsed; an
OpenStep reply is retained and normalized with the platform plist reader.
An actual empty inventory dictionary is valid; empty/malformed output or a
nonzero reply does not establish readiness. Raw CLI help and replies are kept.
Photos bundle presence is recorded as an inventory observation, not inferred
as an import capability.

The probe submits **at most one120-second install**. It then queries actual
registration and both app/data containers, resolves path identity within the
exact owned UDID, and checks the bundle ID, executable name and signed staged
executable SHA256. Up to three bounded read-only container observations allow
a server request to complete after its client timed out. The timeout/signal
record remains unchanged; continuation requires independently verified matching
installed bytes and containers. It is not a second install or a successful
command inferred from empty output.

A matching preexisting reader is verified before skipping mutation. A changed
binary, wrong bundle, foreign container or unresolved state is preserved and
blocked. Explicit installer errors remain distinct from an uncertain client
timeout. Client process-group progress reuses the existing helper. On failure,
a bounded installation-service log query names only the disposable simulator;
there is no service restart, permission workaround, simulator reset, host Photos
write or unrelated process kill.

Installed state is only setup. The unchanged next stages still require Photos
authorization, a baseline, complete PVT URL/file representation, an exact Photos
save callback and independent one-added-Live-asset/original-resources/IDs/cover/
hash readback. No decomposed PhotoKit pair creation substitutes. New native
installation/UI/admission remains unverified until this head's own CI executes.

## Portable checks and actual preceding native evidence

The [author review](392009a-author-review.json) binds all three file hashes.
The [focused regressions](pvt-install-state-final-focused.log) pass45 cases:
client timeout with already-complete or late-complete installation, at most one
mutation, unreadable/malformed service state, unresolved timeout, existing
matching/different binaries, wrong bundle metadata, foreign containers, owned
path aliases and explicit installer error. They use a portable state model and
real temporary files; they do not execute or prove Apple installation.

The initial API-absent RED is retained alongside the actual native first error;
it is not presented as a native rerun. An intermediate all-Python run passed306
before the final alias regression. The [clean final scoped gate](392009a-portable-receipt.json)
passes4/4 with **all307 Python tests**, Rust format, current CLI build and actual
pair/PVT conversion/validation/no-clobber. Its base is exact af223. The unchanged
full Rust/clippy suites were not repeated locally; no old native result is
relabeled as final-head CI acceptance.

The preceding [af223 check snapshot](af223f0-final-checks.json) has all20 checks
completed:19 successes and only the PVT setup failure. Raw native receipts are
now available rather than merely queued statuses:

| af223 evidence | What actually executed |
| --- | --- |
| [Product/App](af223f0-app-observation.json) | Both Xcode builds, all30 model calls,32 package/alias protection tuples, actual macOS debug exhaustive RGB8 consumer test. |
| [Release/performance](af223f0-performance-observation.json) | Exact native release consumer:1passed,0failed,0ignored, then original absolute budgets. |
| [Tone](af223f0-tone-observation.json) | All88 successful nonempty render rows;3products x11tones have finite endpoint data and monotonic luminance. Five reference sweeps remain distinct. |
| [System HEIF](af223f0-system-observation.json) | Three actual native renders with complete comparison data. |
| [Relocated arm64 package log](af223f0-package-job119039472976.log) | Exact-head package build and outside-checkout clean-environment conversion step succeeded; not Photos or wallpaper acceptance. |

af223 absolute Styles median9.749s/p9510.079s/RSS1084.391MiB pass unchanged
11.732s/15.406s/1361.367MiB limits. Its same-runner main8.277483s/candidate
9.755783s remains **+17.8593%**, user8.03/9.21s. Native release bit equivalence
is established; the Linux lookup-kernel speedup does not establish ARM64/E2E
speedup or complete regression attribution. No af223 performance rerun occurred.
Earlier failed attempts remain preserved in v15/v16, not overwritten by green.

## Durable recovery and disabled convergence

The [36-ref manifest](all-36-backup-manifest.json) and
[full restoration log](all-36-backup-restore.log) verify every exact remote
commit/tree, non-shallow full strict fsck0, and all9 current branch recoveries.
The new tag was independently restored before the ordinary source fast-forward.
Main remains89d806c8; the other8 heads did not move. Current native runs remain
pending inspection, with no duplicate manual dispatch of already-running work.

The [convergence plan](392009a-convergence-plan.json) stays disabled: final gates
and parent execution review, ordinary merge71 retaining the original four tips,
actual indirect-merged/automatic-deletion inspection, persistent restored backup
of resulting main, then review of the fresh remaining cleanup set and guard.
No merge/delete, force push, release, upstream write, paid/billing, protection
or global security change occurred. Physical Photos/wallpaper and unavailable
historical PVT sample byte reuse remain unverified.

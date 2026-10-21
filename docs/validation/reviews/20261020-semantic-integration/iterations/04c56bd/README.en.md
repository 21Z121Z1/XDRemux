# Native acceptance and bounded PVT representation controls

Candidate **04c56bda6740212861b4c24907f15bb870e2e997**, tree
**d41d1649a1afd1db45b3b1c81d39337d9ab9b0c2**. This appendix preserves
the completed7a native results and a new, narrowly scoped PVT diagnostic fix.
It does **not** transfer7a CI acceptance to this later source.

## What7a actually passed

The [complete latest-check snapshot](7a561de-final-checks-all-attempts.json)
has19 successes and1 failure, with the original failed performance attempt
retained separately. Both source revision and attempt identities are explicit.

The current-source [native product log](7a561de-product-job119014935829.log)
and [App observation](7a561de-app-observation.json) establish both Xcode builds,
all30 registered model calls and32 distinct package/alias protection tuples.
The source main function was independently counted against the passed harness
sentinel. Every tuple remains blocked before CLI work; source/package bytes,
member sets and symlink targets retain their executed assertions.

The [Tone observation](7a561de-tone-observation.json) retains all88 real render
rows:5 Apple references and3 current product outputs, each11 tones. Every row
has exit0 and nonempty comparison data. All3 product sweeps have finite endpoint
metrics and monotonic mean luminance. Two Apple reference sweeps are themselves
nonmonotonic; there is no universal monotonicity claim. The separate
[System observation](7a561de-system-observation.json) has3 actual native renders
and asymmetric tone response. Neither proves physical Photos editing.

## Performance: both attempts, unchanged budgets

Attempt1 [failed](7a561de-performance-attempt1-review.json): Styles median
13.616s>11.732s and p95 16.015s>15.406s. The same-runner comparison measured
main9.895717s/candidate11.538216s, +16.60%. Candidate CLI, adapter, fixture and
validated profile-output hashes matched prior passing4275 bytes, and both
comparison arms slowed across jobs. This supported exactly one bounded retry;
load alone was not accepted as a cause. No source/input/budget/resource changes.

Attempt2 [passed](7a561de-performance-attempt2-review.json): absolute median
9.608s, p95 9.837s, RSS1083.141MiB under the original limits. Its interleaved
main9.198349s/candidate10.387876s is still **+12.93%**, user8.99/9.97s.
Both full raw logs are retained. There is continuing relative cost, not a
measured performance repair or proof host load explains the entire first fail.
The earlier source-to-work review and rejected HDR-hoisting prototype remain
in the preceding appendix; no correctness rollback or failed prototype adopted.

## PVT: actual first error and smallest next contrast

The [7a native receipt](7a561de-pvt-native-receipt.json) and
[observations](7a561de-pvt-native-observation.json) show cold SDK90.617s,
actual reader3.473s, installation/Photos grant/read-only baseline and UI-test
build all succeeding. The owned iOS27/iPhone11 library contained6 stock assets.
The UI really opened the system share panel. Its tree exposed More/Copy only;
there was no identifiable Photos Save action. The XCTest case failed29.747s,
then xcodebuild remained alive until180s. No save/transfer/after JSON existed.
This is neither a direct import/readback success nor a Photos package rejection.
Source, original package and transported package hashes were unchanged, and
the owned simulator was shut down/deleted successfully. The parent independently
read artifact12319360743; this executor parsed the console receipt and does not
claim it materialized that ZIP.

The six-file04c change records the actual SDK extension/URL/content-provider
type facts, compares a synthetic UIImage activity-availability control (never
saved), an unchanged whole-directory URL and an explicit live-photo file
representation. It selects at most one unambiguous Photos Save, otherwise
records native non-PASS and stops only this invocation's private XCTest process
group. Activities can be accessibility cells as well as buttons. It does not
click unrelated activities or create an unpacked PhotoKit pair.

URL-offered and provider-requested receipts remain distinct. They must match
the current share phase, entire package/member hashes and completed Photos
activity. A previous phase's transfer record is removed before another offer.
Independent one-Live-asset, original photo/paired-video, pairing identifier,
cover time and resource-byte readback is still required. Missing Save also
attempts read-only post-state observation without turning it into admission.

Apple's public UTTypeLivePhoto page identifies com.apple.live-photo as an image
type; it does not establish that a directory of this layout is its accepted
serialization. The actual SDK facts are collected rather than inventing a new
UTI. Official documentation pages required JavaScript, their Markdown fetch
was unsupported by the browsing tool, and executor HTTP was403; the empty
source-download manifest is retained instead of claiming full API text access.
References: [UTTypeLivePhoto](https://developer.apple.com/documentation/uniformtypeidentifiers/uttypelivephoto),
[PHLivePhoto](https://developer.apple.com/documentation/photos/phlivephoto),
[file item provider](https://developer.apple.com/documentation/foundation/nsitemprovider/init(contentsof:)).

The old simctl addmedia directory rejection is not repeated. macOS Photos
isolated-library import, real-device Photos, wallpaper eligibility and byte-level
reuse of the lost historical sample remain unverified.

## Exact portable evidence and safe convergence

The [clean committed04c gate](04c56bd-portable-receipt.json) passes7/7,
including all295 Python checks and the current real CLI pair/PVT roundtrip and
no-clobber journey. Linux Rust retains the two explicitly named JPEG CLI
exclusions; native product CI retains the complete suite. Focused checks pass33.
One wrong test-module invocation and one wrong gate argument are retained as
failed setup evidence, not passing tests. The [author review](04c56bd-author-review.json)
binds all6 changed-file hashes and explains each accepted finding. Product
crates, production Swift/App, codecs and performance budgets are byte-identical
to7a. Actual new native Swift/UI/functional CI is pending this candidate.

The [33-ref manifest](all-33-backup-manifest.json) and
[full restoration log](all-33-backup-restore.log) bind exact remote commit/tree
identities, non-shallow full strict fsck0, and all9 current head recoveries.
The new source tag was restored before its ordinary integration-branch push;
the other8 heads stayed unchanged. [Convergence remains disabled](04c56bd-convergence-plan.json):
ordinary merge71 after final gates and parent execution review, verify original
58/69/65/68 ancestry and actual indirect merged states/auto deletion, persist
and restore resulting main, then review the fresh remaining deletion set/guard.
No merge/delete, hook activation, force push, release, upstream write, billing,
protection relaxation or global security change occurred.

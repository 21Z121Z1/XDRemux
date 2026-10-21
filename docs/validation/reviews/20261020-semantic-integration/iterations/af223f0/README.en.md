# Cold SDK lifecycle and exact RGB8 decode reuse

Candidate **af223f00856b31756559a70ad59abb5af0bdb462**, tree
**3ae746a69fb1896718dba40ae5b8bb6f7526762f**. It contains two focused
commits after04c:1286910 repairs native setup;af223f0 removes repeated RGB8
decode work and adds a native release equivalence requirement.128 was backed
and locally verified but never pushed as a separate PR head. The source branch
advances once from04c toaf223 to avoid another intermediate CI batch.

## Actual new failures, retained

The [04c PVT receipt](04c56bd-pvt-native-receipt.json) stopped at the unchanged
cold SDK canary:120-second ceiling,122.340s observed, empty stdout/stderr.
The same canary previously compiled in90.617s on7a. Reader compilation,
installation/authorization, baseline and all three representation controls
were not reached. This does not prove SDK incompatibility or Photos rejection.
Current CLI generation/package validation passed, source/package hashes did
not change, and the owned simulator was shut down/deleted successfully.
[Full log](04c56bd-pvt-job119029842052.log); existing artifact12320510762 was
not rerun merely to retrieve its empty stderr.

The [04c performance attempt](04c56bd-performance-observation.json) failed:
Styles median12.130s>11.732s; p9514.774s<=15.406s and RSS1084.109MiB within
budget. Same-runner main9.302207s/candidate10.567903s is+13.6064%, not a
resolved cost. No rerun was requested. The preceding7a failed first attempt
and single reasoned passing retry remain in v15; a latest green did not erase
them. All original absolute budgets and comparison samples remain unchanged.

## Setup change and evidence boundaries

Cold SDK preparation, actual reader compilation and signing now happen before
simulator creation/boot. No simulator needs cleanup if compilation fails before
creation. Cold SDK imports have a240-second ceiling; actual reader120s and UI180s
remain distinct. This addresses observed cold-start proximity to120s while
avoiding competition with an idle booted simulator; it is not a measured claim
that simulator load caused the timeout.

Every ten seconds the compile records only its private process group's names,
state, CPU and elapsed time. No argv/environment or unrelated process details
are uploaded. Timeout stops the owned session and retains the phase trace,
stdout/stderr, Swift/Xcode/SDK identity and non-PASS outcome. UI phase/count
observations are also copied into the compact command receipt so a bounded
console tree excerpt cannot hide a middle control observation. Missing Photos
Save, failed readback or cleanup cannot become a pass.

The [focused regression](pvt-cold-setup-before-fix.log) records the original
compile-after-create order failure and absent owned-process API; this is not
a fabricated native rerun.35 focused tests and297 Python tests pass. The
intermediate128 clean gate passed7/7 and its exact durable tag was restored.
New Swift/native setup, controls and actual package admission remain pending
the final source's own CI.

## Bounded CPU prototype and adopted change

The validated source is Raster8/RGB8, so its channel domain has exactly256
values. The old loop calls the same scalar sRGB decoding formula for every
component. The candidate computes that formula once per code into a1KiB array,
then indexes it using the actual u8 value. It does not alter the transfer
equation, FP arithmetic, HDR-before-reduction order, gain-code interpolation,
gamma/headroom/offset handling, RGB16/Main10 path or any validation.

The [audit prototype](sdr-lut-prototype/result.json) copied the exact old scalar
function. On Linux x86_64 it checked all256 inputs and all150,994,944 components
decoded locally from the exact public fixture, bit-for-bit. The8192x6144 RGB8
input, libheif stride/options, compiler and hashes are recorded. Eight
alternating-order measured samples per arm after one warmup include table
initialization and output allocation, exclude output deallocation. Kernel
medians were1.809729s scalar and0.310565s lookup, ratio0.171609. This is a
standalone kernel result, not actual runtime/native/E2E acceptance or causal
attribution of the entire main/candidate gap. Decoded pixels, source media and
compiled executables are excluded from this public evidence.

The native actual-consumer regression drives the real Styles scene path with
all codes in every RGB channel, channel permutations and padded rows. It
compares actual scene float bits to the original scalar operations at runtime;
black_box prevents constant-folding the scalar reference. The performance job
must run this exact test in release and observe one passed, zero failed and
zero ignored tests before measuring normal workloads. It does not replace
the end-to-end budget or the ordinary complete native suite.

The attempted [Linux consumer command](sdr-lut-consumer-scalar-baseline.log)
exited0 but executed0 matching tests because apple_styles is cfg(macos).
It is **not a passing consumer baseline**. The [missing-test guard negative](sdr-lut-zero-tests-rejection.log)
rejects that actual log.51 focused workflow/performance checks pass; native
release compilation, exact bit identity and ARM64 performance remain unverified
until the final CI executes them. No old-head result is substituted.

## Machine constraints and convergence

| Repeated failure pattern | Concrete boundary or automatic check |
| --- | --- |
| Cold compiler timeout with no useful stderr | Pre-boot compile; bounded private-session lifecycle and owned-process progress facts. |
| Missing Save/callback mistaken for import acceptance | Phase-bound whole package, exact activity, independent one-Live-asset/resources/ID/cover/hash readback. |
| Busy-runner reruns hiding the original red | Retain all attempts, unchanged absolute budgets and rotated same-runner main/candidate comparison; no04c rerun. |
| A successful command ran no relevant native tests | Exact native release test and explicit one-passed/zero-ignored evidence guard. |
| Repeated decode of the same finite RGB8 domain | One256-entry typed array; actual-consumer bit equivalence required before acceptance. |

The [final clean portable gate](af223f0-portable-receipt.json) passes7/7,
including297 Python checks and actual current CLI pair/PVT roundtrip/no-clobber.
Linux Rust retains two explicitly named repeated JPEG encode exclusions;
native product CI retains the complete suite. The [author review](af223f0-author-review.json)
binds all six changed files since04c and names native gaps. This is author
review, not independent execution of Apple code on Linux.

The [35-ref manifest](all-35-backup-manifest.json) and
[complete restoration log](all-35-backup-restore.log) bind every exact remote
commit/tree and non-shallow full strict fsck0, covering all9 current remote
heads. Both intermediate128 and finalaf223 tags were independently restored
before advancing the source branch. Main remains89d806c8 and the other8 heads
did not move. [Convergence remains disabled](af223f0-convergence-plan.json):
all final native/functional gates and parent execution review precede an
ordinary merge71, actual indirect merged-state/auto-deletion inspection, durable
restoration of resulting main and a fresh reviewed cleanup set. No merge,
deletion, force push, release, upstream write, paid/billing, protection or global
security change occurred. Earlier review/skill/disposition evidence remains in
the inherited appendices; no new general framework or permissions were added.

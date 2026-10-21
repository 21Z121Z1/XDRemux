# Refuse borrowed object stores before retaining a reference clone

Product e107 and all product/CI configuration remain unchanged. This correction
adds one object-store invariant and one real borrowed-source fixture to the
accepted v25 portable prototype. No native compilation, PVT/performance run,
rerun, dispatch, product-head push, merge/delete or permission change occurred.

## Minimal refusal policy

The helper now checks the effective Git object directory before executing the
fixed comparison owner, immediately before cloning and again in the retained
clone before checkout/build. Nonempty Git object/worktree directory redirections
are refused. Presence of `objects/info/alternates` or `http-alternates`, including
empty or dangling-link metadata, is refused. The source is not repacked,
dissociated, edited or deleted. A clone already made when a postcheck fails
remains retained. Each check and its result is checkpointed with bounded Git
commands. This chooses source refusal rather than a source-rewriting repair.

`--no-hardlinks` still prevents hardlink copying; it is no longer used as evidence
that borrowing stopped. Git's [clone manual](https://git-scm.com/docs/git-clone)
and [repository layout](https://git-scm.com/docs/gitrepository-layout) distinguish
local object copying from alternate-store dependencies. These are observations
at the checked boundaries, not a lock against later writers or a new claim of
hermetic/shallow/promisor compatibility. Existing exact revision/clean-source,
retained ownership, independent subprocess deadlines and uncertainty rules stay.

## Real failing-before and passing-after evidence

The additional fixture creates a lender and an actual `git clone --shared`
borrower. The borrower can read its commit while lacking that loose commit
object locally, and has a real alternates file. It is not a plain git-init
positive mislabeled as borrowing.

[Before receipt](borrowed-before.json) and [log](borrowed-before.log) record exit1
against the exact v25 helper hash. The old helper made another retained clone
which inherited alternates and reached a deliberately stubbed build boundary.
The [raw fixture receipts](borrowed-before-receipts) retain that observation.
No Cargo/Apple code ran. Lender, borrower and owned clone were not deleted.

[Final portable checks](portable-verification.json) passed22/22 in place and
22/22 from a copied non-Git directory with spaces/Chinese characters. Only one
new scenario was added. The corrected path rejects borrowing before creating
a clone/build or executing the owner. Existing ordinary-clone and lifecycle
checks also pass. Fixtures, bounded-process fault tests and Git checks remain
portable evidence. Rust/Swift native compilation, macOS stage execution and
stage-cost results remain unverified. v25's scope, release-time/logging-overhead
caveats and native/media constraints still apply.

## Public raw independent recovery evidence

The previously local [v25 original fetch/restore/fsck transcript](v25-original-restore.log)
and [publication receipt](v25-original-publication.json) are now included. They
name exact ref `refs/tags/review/semantic-integration-20261020-evidence-v25`,
commit64fb7e5c5193698e88e8ba035be3c809f7c38763 and
tree9f5214d0e7ba50a4d939b4658133be79194cf5f8, with command exits. The original
transcript used the literal `<restore>` label; the current actual path is
`/workspace/xdremux-audit/restore-test.git`.

[New read-only verification](v25-existing-independent-verification/report.json)
and [full actual-command transcript](v25-existing-independent-verification/commands.log)
show that existing independent bare store has no alternate metadata, exact
commit/tree, non-shallow state and full strict fsck exit0. Individual stdout and
stderr files are retained unmodified. No network fetch was repeated there.
This verifies v25 evidence, not a new product/native acceptance result.

A separate attempt to fetch v25 from scratch into a new empty bare repository
[timed out](v25-fresh-recovery/report.json) after120.002s with blank fetch stderr.
The leader exited-9, but its process-group absence was not confirmed. A subsequent
read-only ref query returned128/absent; fsck was never run there. A bounded
PGID-only observation found zombie Git descendants adopted by PID1. The original
cleanup receipt remains **uncertain**, not retroactively green. That store and
all partial data are retained; no extra signal, deletion or fetch retry occurred.
Command rows/streams are original; top-level terminal fields annotate the timeout
and subsequent read-only check (the initial one-off script had only a starting
root checkpoint). Do not substitute this unsuccessful attempt for the separate
successful original recovery/current local fsck evidence.

The newly published fix's exact recovery transcript will be added as a separate
receipt-only evidence commit, avoiding a self-referential commit/hash claim.

## Native entrypoint inventory, not an execution request

[Exact current workflow](performance-workflow-e107.yml) and
[entrypoint requirements](native-entrypoint-requirements.json) are read-only
snapshots. `.github/workflows/performance.yml` at
`integrate/semantic-review-20261020`/e107 has `workflow_dispatch` with **no inputs**,
job `rust-performance`, label `xcode-27`. It runs the existing absolute benchmark
and candidate-only comparison; it does not call this corrected archived
prototype. Dispatching an evidence tag would not wire the prototype either.
Neither is a usable entrypoint for this correction, and neither was triggered.

The selected environment is Linux without xcodebuild/Swift. No callable dispatch
tool is exposed. No server-side dispatch rejection or Actions-write denial is
established; the old gh read-only narrative still lacks a complete archived
receipt. No gh retry or alternative403 artifact download was attempted.

A future explicitly reviewed performance-only entrypoint would need the exact
product SHA e107, fixed reviewed harness SHA, an explicit diagnostic-only mode,
a free standard arm64/macOS27 allocation, actual Xcode/host facts, separate clean
full-history product/harness checkouts and fresh release CLI/adapter build hashes.
It must pass the prototype's existing six CLI inputs: source, CLI, adapter, their
two build hashes and a fresh text-output directory. Retained clones/media stay
outside text artifacts. It must preserve prior failed budgets and run no PVT.
Those dispatch inputs/mode do **not exist** in the current workflow. No workflow
change or alternative trigger is implemented or authorized by this document.
The exact resolved harness SHA is supplied by the publication recovery attachment
for parent review. No native method is currently available through this session.

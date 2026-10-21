# Manual Styles diagnostic entrypoint: static review

The performance-only entrypoint is fixed at **33bbe860ad6b1777839753a738ec17d3f9229b61**,
tree **bfd8a388308c0e3b6e3e59a6065abb54247e37cc**, under the additive immutable
tag `review/styles-stage-diagnostic-20261021-v1`. It is a separate child of product
e107, with six changed files. Main and PR71's e107 source remain unchanged.
No native run, dispatch, rerun, new PR, merge or deletion was performed.

The exact workflow/ref/input request is [dispatch-request.json](dispatch-request.json).
It is **prepared, not submitted**. The existing default-branch `performance.yml`
registration is reused. The new job requires `workflow_dispatch`, explicit
`styles-stage-attribution`, the exact tag, the matching full entrypoint SHA and
public own-repository metadata. Ordinary push/PR cannot run it; diagnostic mode
cannot select the old performance job. The review tag cannot select the old job
even in standard mode. The old job's steps, runner and limits match the exact
e107 fixture. Its concurrency group is retained at job scope; diagnosis has a
distinct per-run, non-cancelling group. No PVT job/dependency is added.

## What would run after review

The new job uses only the existing standard public `xcode-27` arm64 configuration
and `contents: read`. It does not introduce a larger/self-hosted runner or change
billing, permissions or security settings. It checks actual arm64/macOS27,
Xcode27/SDK27 facts. Runner classification is inherited from the previously
authorized execution configuration; the label alone is not a new free-resource
proof. Reconfirm that configuration before eventual execution.

Three separate full-history checkouts hold control, product
`e1072a90e31b12861dfe8b9cf25e07ce75c17c44` and harness
`1a1698e47e008b688cf5c3fc492dfc307b04ae1a`. Clean identity, no alternates,
separate storage, tracked file bytes/modes and the two literal reviewed Python
hashes are checked before harness code executes. The accepted helper/prototype
are unchanged. Fresh Cargo home/target and Swift scratch directories prevent
reusing an old product binary. Command/environment/toolchain, feature graph,
architecture and actual new CLI/adapter hashes become receipts. There is no
Actions product cache. Source/binary bytes are rechecked after execution.

Preparation is bounded to 15 minutes; the accepted in-process harness keeps its
own 90 minute budget and cleanup. Step/job limits are 110/120 minutes; final
source checks get 60 seconds and bootstrap/stop commands get 20/2 seconds.
Partial and failure receipts survive when possible. Initial text exists before
checkout. An explicit UTF-8 text whitelist exports only owned flat JSON/log files,
with size and credential checks. Clones, converter stdout, media and binaries
remain outside artifacts. Upload has not been tested on GitHub. Uncatchable
host loss may preserve only the last checkpoint, not prove cleanup.

## Evidence actually obtained

At the fixed entrypoint commit, **65 portable tests passed**, including 14 new
tests and 51 existing performance/workflow checks. See the [command, source
hashes and log hash](styles-dispatch-committed-test-receipt.json) and [raw test
log](styles-dispatch-committed-tests.log). The [14-case matrix](static/event-mode-matrix.json)
evaluates the real job expressions in a bounded static subset; it is not GitHub
execution. Compiler/architecture outputs in the receipt test are explicitly
modeled. The Linux driver entry really refuses before checkout/build/harness.
The first development run's output-directory test failure is retained in
`styles-dispatch-tests.log`; it was corrected before the committed 65-test run.

[Fixed Git blob proof](styles-dispatch-fixed-blob-proof.json) compares both
harness literals against 1a169's actual Git bytes and the legacy workflow fixture
against e107. [Preflight raw commands](preflight-raw/report.json) inventory exact
workflow bytes at default main, entrypoint and prior evidence, current 9 heads,
local worktrees and the unchanged historical dirty file/patch. No old patch was
applied. [PR71 metadata](styles-dispatch-pr71-state.json) confirms draft/open e107.

All six workflows in each inspected tree have only `push.branches: [main]`, no
tag filters, `create` or `workflow_run`. Under GitHub's documented undefined-ref
type rule a tag push cannot select them; path filters are not used as a tag guard.
Default CodeQL setup scans default/protected branches. Sources:
[push branch/tag rules](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onpushbranchestagsbranches-ignoretags-ignore),
[existing workflow dispatch registration and ref](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch),
[default CodeQL setup](https://docs.github.com/en/code-security/concepts/code-scanning/setup-types).

After that preflight, one normal additive tag push saved the entrypoint. Its
[raw publication/recovery commands](publication-restore-raw/report.json) record
the exact remote ref, commit/tree, independent non-shallow no-alternate restore
and full strict fsck exit 0. The raw stdout/stderr files are adjacent. The
existing independent bare repository is used; no new full-fetch timeout is hidden.
[The all-event GitHub run query](styles-dispatch-server-runs-initial.json) found
zero runs at that SHA after publication. This is a time-stamped observation, not
an indefinite server guarantee. It uses the all-event runs endpoint rather than
the PR-only convenience wrapper.

## Review result and remaining boundaries

This is an **author review**, using the project's scoped pstack workflow,
engineering principles and review rubric. No independent reviewer or new model
session was created. Boundary discipline produced fixed source bytes, fresh
outputs and receipt/export ownership; direct verification separates models,
Linux refusal and future native behavior. The accepted prototype was not polished
or replaced. The final review found no additional demonstrated in-scope blocker
after the portable checks. No native success is inferred from this review.

The current session has no callable workflow-dispatch tool. No dispatch POST was
attempted. No Actions-write denial is established; unsupported read URLs and the
older incomplete gh Forbidden record are not dispatch authorization evidence.
Server input validation, macOS builds, measurement and artifact upload all remain
unexecuted. A parent-reviewed tool/operator can submit only the stored exact
request later, after confirming standard free runner availability.

The e107 **18 green/2 red** terminal state and all older failures remain. Styles
median 14.458/p95 16.156 exceed the unchanged 11.732/15.406 seconds; same-host ABBA
main 12.434200333/candidate 14.704374375 is +18.2574993%. No stage result or
optimization claim exists. Raster/gain_raster lifetimes and nested logging differ;
do not add overlapping spans or stage medians as E2E attribution. PVT remains
blocked at simulator services, product calls 0, cleanup uncertain; this entry has
no PVT path and gives no Photos admission result. Full-package saving/readback
and wallpaper claims remain separate. Main/PR heads, budgets, strict rules and
`operation_enabled=false` remain unchanged.

# Pinned manual Styles stage diagnosis

This review-only entrypoint uses the existing `performance.yml` dispatch
registration. It is not a product budget gate. Main and PR71's product source
remain unchanged. No dispatch or native run is part of implementing this entry.

Use only the immutable review tag
`review/styles-stage-diagnostic-20261021-v1`, after reviewing its exact commit.
The future dispatch inputs are `mode=styles-stage-attribution` and
`reviewed_entrypoint_sha=<that full entrypoint commit>`. Product and harness are
not configurable inputs: they are fixed to
`e1072a90e31b12861dfe8b9cf25e07ce75c17c44` and
`1a1698e47e008b688cf5c3fc492dfc307b04ae1a` respectively. An execution request with
the resolved entrypoint SHA is stored in the separate static review evidence.
Do not submit it until parent execution review has completed.

## Event and resource isolation

Ordinary push/PR events cannot select the new job. Its guards require the
explicit diagnostic mode, exact review tag, reviewed entrypoint SHA equal to
GitHub's SHA, this repository and public-event metadata. Normal budget dispatch
and ordinary events retain the existing `rust-performance` steps and limits;
the review tag cannot accidentally select that legacy job even in standard
mode. Invalid or missing diagnostic inputs do not allocate the diagnostic job,
and the runtime rechecks the request before effects. Skipped jobs are not
acceptance evidence.

The diagnostic has its own per-run concurrency group with cancellation disabled. It
cannot cancel the ordinary performance or research workflows. It has no PVT job,
reusable-workflow call or cross-job dependency. `contents: read` remains the only
permission. It uses the existing standard `xcode-27` label; the runtime requires
actual arm64/macOS27, Xcode27 and SDK27 facts. Public repository status and the
standard/free runner classification must be verified before future execution.
No larger/self-hosted runner or billing, protection or security change is made.

## Fixed code and fresh product outputs

Control, product and reviewed harness use separate full-history checkouts with
credentials persistence disabled. Before any harness Python executes, bounded
bootstrap commands check exact SHA, checkout/Git/object-store identity, clean
tracked/untracked state and absent alternate metadata/redirection. Every tracked
working-file byte is checked against its Git blob ID, not merely `git status`.
The two reviewed Python files also match fixed SHA256 values. Only then are
those exact bytes loaded. The accepted prototype/helper are unchanged.

Preparation has a 15 minute deadline including verification, libheif/Rust setup,
fresh release Rust/Swift builds and the candidate feature graph. Candidate
Cargo target, Cargo home and Swift scratch are newly created empty directories;
there is no Actions product cache and no supplied old binary path. Wrapper,
custom target and Rust flags are refused. Build commands, selected environment,
tool versions/paths, graph hash, arm64 architecture and actual CLI/adapter hashes
are recorded, including partial failures. Product tracked bytes are rechecked
after the build and after diagnosis; binaries are hashed again. The toolchain or
SDK may already exist: this is not a cold-host claim.

The reviewed CLI runs in the same Python process with its original 90 minute
budget and ownership/cleanup behavior. This avoids an outer SIGKILL stranding
its separate command sessions. Preparation and final byte checks are separate
from all measured samples. The step/job limits are 110/120 minutes; final checks
have 60 seconds and individual bootstrap/stop waits have 20/2 seconds. A timeout,
interruption, unknown cleanup or missing report remains failed/incomplete.
Retained clones, media and build outputs stay outside text artifacts. Host loss
or an uncatchable kill can leave only the last checkpoint; no cleanup success is
inferred from missing data.

## Text-only evidence

An initial owned receipt exists before checkout. The driver creates a separate
fresh artifact directory and emits readiness only after owning it. Uploads use
flat `.json`/`.log` paths in that one directory, plus the separate exact initial
receipt. No recursive raw/checkout/clone/media upload occurs. Export is limited
to named receipts, selected build/feature logs, CPU/stage stderr, validation logs
and the timer patch. Converter stdout dumps, images, movies and pixel buffers
are excluded. Symlinks, binary/non-UTF8 content, oversized files and possible
credential text are withheld with an explicit failed-export manifest. Original
raw files remain local. Limits are 4 MiB per text file, 32 MiB total and one-day
artifact retention. Full environment or event payloads are never dumped.

Partial receipts and failed attempts are retained. Upload failure is not proof
that native assertions passed. A diagnostic completion does not erase the
existing absolute-budget failures or satisfy the PR's required gates. Stage
comparisons explain broad operations: raster/gain_raster release scopes and
nested logging costs differ. Do not sum overlapping spans or stage medians as
E2E attribution, or identify a performance cause before measurements exist.

## Current evidence boundary

Only YAML/expression matrix, source-pin, fresh-directory/receipt models, export
boundaries and the actual non-macOS refusal are checked locally. Existing
performance/workflow policy regressions are also checked. Model compiler output
is labeled as a model and is not retained as a real build receipt. No macOS
compilation, stage measurement, server dispatch/input-schema validation or
artifact upload has executed for this entrypoint. The current session exposes
no workflow-dispatch tool. This is not evidence of an Actions-write denial.

GitHub documents [dispatching an existing workflow against a branch/tag](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch).
Its [branch/tag event rules](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onpushbranchestagsbranches-ignoretags-ignore)
exclude tags when only branch filters are defined. All six repository push
workflows are main-only; review-ref publication must first verify this inventory
again. [Default CodeQL setup](https://docs.github.com/en/code-security/concepts/code-scanning/setup-types)
scans default/protected branches, not this tag. These static facts do not claim a
server run has occurred.

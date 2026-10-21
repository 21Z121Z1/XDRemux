# Portable stage prototype with retained ownership and bounded commands

This supersedes the v24 prototype's portability/lifecycle design, not its
accepted e107 terminal CI archive. Product head remains
`e1072a90e31b12861dfe8b9cf25e07ce75c17c44`. No product source, performance budget,
workflow, permission, protection, main or branch was changed. No native experiment,
dispatch request, rerun or source-head push was made. Merge/delete remain disabled.

## Identity before executable code

The CLI requires an explicit source checkout. Before loading the comparison
owner, bounded Git commands verify its resolved checkout root, exact fixed SHA,
Git directory identity and clean tracked/untracked state. The owner Python and
candidate Rust are read from that fixed SHA with `git show`, preserving exact
UTF-8 bytes including trailing newlines. The checks are repeated before code
execution, and source identity/hashes are checkpointed first. The frozen Python
blob is executed, not imported from a mutable worktree file. Its ROOT is pinned
to the verified checkout after execution. The fixed Rust blob supplies timer
insertion; no working-file substitution is allowed. A later writer cannot change
those frozen bytes. A clean check is not a lock: the final source check still
rejects drift, and it does not promise to prevent another writer's changes.

Fixture tests use adjacent exact source snapshots with [provenance hashes](fixture-provenance.json),
not `/workspace` locations. Isolated Git repositories exercise actual SHA/root,
dirty-tree, between-read and late-drift cases. Wrong SHA/root or a dirty tree
prevents owner execution. The late-drift test distinguishes fixed executable
bytes from changed working files; it does not label the changed checkout clean.
The main KeyboardInterrupt test explicitly records `interrupted`.

## Remove shared-worktree cleanup from this execution path

The old prototype used the comparison owner's linked worktree and finally
cleanup, including unbounded Git list/remove and Popen waits. This version does
not invoke that reference_build path. It makes a fresh independent local clone
with `--no-hardlinks`, no object alternates, separate Cargo target/Swift scratch,
and no shared worktree registration. The clone, outputs and targets live outside
the text-evidence directory. They are **retained by design on success, failure
and interruption**. There is no worktree list/remove, force removal, reset or
recursive deletion in the prototype path. The canonical product/comparison
owner is unchanged; only the helpers actually used here get bounded Git/process
implementations. This is not a claim that the canonical owner's cleanup was
changed everywhere.

Timer installation first checkpoints a recovery location and renames the old
owned-clone source inode into it. It checks the retained bytes and exclusively
creates the patched leaf. Concurrent changed bytes are preserved in the backup;
a new leaf causes refusal instead of overwrite. An interruption between rename
and creation can leave the clone's leaf absent: the retained source location is
in the receipt, and no automatic restore risks overwriting another writer.
These mutations affect only this invocation's independent clone, never the input
checkout. Review retained paths before any later manual cleanup; the prototype
does not delete them or claim workspace cleanup success.

## Independent deadlines and incomplete outcomes

Every Git observation has its own30s budget; clone/checkout have120s; builds and
native commands retain their existing bounded budgets. The90minute overall
alarm is additional. On timeout or interruption, a command's fresh process
session is stopped with SIGKILL and a separate2s bounded reap. Handled signals
are deferred during that stop/receipt interval. No Popen context manager or
unbounded wait is used. If the leader was already reaped, no signal is sent to
its possibly reused PID/group. Remaining groups, a failed reap, or an interrupted
start without a returned process handle are **uncertain**. No PID is guessed,
no global process is reset and no retry is implied. Each command checkpoints
its phase, budget, stdout/stderr paths, exit/error, elapsed time and independent
termination result. Main records interrupted/timed-out/failed plus an uncertainty
summary; absence of a handle is not reported as proof that nothing started.

The source clone and all sample media remain local outside text artifacts.
Receipts retain their locations. Only explicit text allowlists may be archived;
do not upload a parent directory containing retained media or clones. A host
loss or SIGKILL can leave only the last checkpoint. These subprocess deadlines
do not prove that a non-returning kernel/filesystem operation is recoverable or
that an uncertain process has stopped. They never turn uncertainty into success.

## What actually ran

[Portable verification](portable-verification.json) ran21 tests twice on Linux
Python3.12: once here, once from a copied bundle in a non-Git directory with
spaces and Chinese characters. The source/PYTHONPATH overrides were removed.
Both runs passed. No original checkout or audit path is required by the copied
bundle. The source snapshots are Rust text fixtures, not media.

[Fault receipts](failure-injection-summary.json) retain real subprocess timeout,
nonzero exit7/stderr, real SIGINT, an injected KeyboardInterrupt after Popen,
and overall alarm expiry. Each preserves a simulated user change and records an
independent bounded stop. A deliberately uncertain reap is kept uncertain even
after the test-only drain; an interrupted start has no guessed PID. The actual
local-clone/build-failure fixture confirms the source worktree list is unchanged
and user bytes/independent clone remain. Timer-install concurrency negatives
preserve changed bytes and refuse an existing new leaf. The Git fixture does not
run Cargo or Apple code. Tests have explicit POSIX skips; the actual non-macOS
CLI refusal is skipped on Darwin, not asserted to be a Darwin result.

The test suite was added with this correction; no pre-fix failing execution was
captured. The unbounded old paths were established by source review. The new
fault tests are executable observations of this implementation, not a claim of
native reproduction.

| Evidence | Current boundary |
| --- | --- |
| Portable tests/relocation |21+21 passed on Linux; POSIX/process and isolated Git fixtures |
| Rust/Swift native compilation |Not executed; still unverified |
| macOS prototype execution |Not executed; still unverified |
| Stage cost comparison |No measurements obtained |
| PVT/performance CI |No new run; accepted v24 e10718green/2red archive unchanged |

## Interpretation and execution capability

Even a future valid native stage comparison explains **broad operations**.
The two versions release raster/gain_raster resources at different scope points;
a stage may include destruction while another retains that memory into later
stages. Candidate nested logging has different overhead. A scene delta therefore
cannot be called pure reconstruction/resize algorithm cost. Nested spans must
not be added to their enclosing scene span, and stage medians must not be added
as E2E attribution. No optimization or performance result is claimed here.

[Execution-capability correction](execution-capability-note.json): no callable
workflow-dispatch tool is available, which says nothing about a server rejecting
a dispatch. The historical gh read-only Forbidden narrative lacks a complete
retained endpoint/exit-code/stderr receipt. It does not establish denied Actions
write permission. No dispatch POST, read retry, alternative403 artifact download,
rerun or source push was attempted in this round. These evidence gaps remain
explicit; they are not converted into permission/Actions conclusions.

Run only the portable bundle with:

```sh
python3 verify-portability.py
```

A later native operator must first review the fixed prototype/helper hashes and
provide the exact clean e107 checkout, fresh binary build receipts and an owned
text-output directory. Native execution is not authorized by this README and was
not started in this correction. The existing CLI arguments are unchanged;
retained workspace paths and recovery receipts now require explicit review.

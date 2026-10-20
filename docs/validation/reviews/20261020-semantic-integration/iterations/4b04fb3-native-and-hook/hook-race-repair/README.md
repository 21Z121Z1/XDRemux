# Cleanup guard: original-hook race repair, preparation only

**NOT-APPROVED. No production cleanup, GitHub push, persistent hooksPath change, or production permission change was performed.** The rendered production review artifact remains mode `0644`, so Git cannot execute it as a hook. The existing 4b04fb3 candidate manifest remains an unapproved historical snapshot; it is not a fresh authorization or an assertion about a later product head.

This directory supersedes only v7's generic original-hook chaining proposal. The original v7 sources and 16-case evidence remain unchanged in the parent directory. Complete old source copies are in `v7-source-snapshot/`. Do not use the v7 launcher for a future cleanup.

## Observed environment and narrower contract

The read-only inspection in `actual-hooks-read-only.json` checked only `core.hooksPath`, Git directory discovery, and filesystem metadata. In `/workspace/XDRemux`, `core.hooksPath` was unset, the effective directory was `/workspace/XDRemux/.git/hooks`, `pre-push` was absent, and all 14 entries were ordinary `.sample` files. The repository, `.git`, and hooks directory were UID/GID 1000, mode `0700`. No configuration or permissions were changed.

The supported behavior is therefore **no active original hooks**. Preparation refuses any original `core.hooksPath` setting and any directory entry other than an ordinary `.sample` file. This deliberately also refuses non-executable hook files, symlinks, directories and unknown names. Any real original hook requires renewed parent review; this implementation never forwards or executes one. It preserves the actual absence of original hook behavior, and does not claim general hook compatibility.

Preparation pins the original Git directory and hook directory identities and permission metadata. Immediately before checking the advertised deletion batch, the launcher checks that it is in that Git directory, that the only hooksPath value is the expected command-scoped temporary override, that Git resolves that override to the expected temporary directory, that pinned directory identities and permissions still match, and that the original directory still has no active hooks. Owners must match the current UID; group/other-writable directories are refused. The decision to reject an observed hook uses the captured inspection result, with no later `exists()`/path-exec branch.

`guard.py`'s `validate` function is byte-for-byte unchanged from v7 (`deletion-validator-equivalence.json`). It still requires the exact complete deletion set, zero local OIDs, exact transport-advertised remote OIDs, non-main branch names, one matching push URL, and pinned manifest/guard hashes. The launcher executes the same guard bytes that it hashes.

## Real local transport evidence

The command actually run was:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 /workspace/xdremux-audit/cleanup-guard/hook-race-repair/exercise_local.py
```

The final successful run is `local-fixtures-74goh9xg/`; its selected text evidence is copied into `evidence/`. `commands.jsonl` contains the exact local commands, working directories and exit codes, and `results.json` binds the run to all four repair script hashes. Every writable repository was a disposable bare fixture underneath this repair directory. Transport was restricted to `file`, and system/global Git configuration was disabled only for fixture subprocesses. No commit was created. The seed's 1,314 commit objects exactly match the ancestor closure of the already backed-up main `89d806c8aec961a7991eb3e6d6e2b1370aed8791` and integration `1b2c07da2cb2b0c89c64ca546148fe5d68d75a97` tips.

| Case | Observed result |
| --- | --- |
| v7 deletion after hash check | Bug reproduced: original rejecting hook was skipped; local push exited 0 and removed both fixture refs. |
| v7 replacement after hash check | Bug reproduced: replacement bytes executed; local push exited 0 and removed both fixture refs. |
| New guard, no active original hooks | Local atomic push exited 0; exactly the two approved fixture refs were removed; main and control tag remained. Renderer had left the hook `0644`; only this local fixture explicitly activated it. |
| Prepare with a real original hook | Preparation exited 1 before creating output. No push or hook execution occurred; both refs remained. |
| New guard, observed hook removed before rejection | Push exited 1; captured unsupported state was rejected; original/replacement bytes were not executed; both refs remained. |
| New guard, observed hook replaced before rejection | Push exited 1; captured unsupported state was rejected; original/replacement bytes were not executed; both refs remained. |
| Additional original hooksPath setting | Push exited 1; the unexpected scoped configuration was rejected; both refs remained. |
| Original hook directory permission change | Push exited 1; changed mode was rejected; both refs remained. |
| Original hook directory replaced with another empty directory | Push exited 1; changed identity was rejected; both refs remained. |

These are deterministic interleavings in real Git pushes. Fixture-only trace wrappers call the exact rendered old launcher or exact hashed new guard and perform removal/atomic replacement after the relevant inspection has completed. They are not production launchers. Inspect the copied `effective-test-pre-push.py`, `reviewed-launcher.py`, `race-interleaving.txt`, packet logs, and before/after JSON rather than treating the table as the evidence. Absence of execution markers is asserted by the harness and recorded in each result.

All intentionally deleted fixture refs were restored with `git update-ref <ref> <backed-OID> <zero-old-OID>`, and complete ref maps were compared to their pre-push state. `restore.json` and `commands.jsonl` record that recovery. All original v7 file hashes still match (`v7-preservation-after.json`).

The first harness attempt injected using an unexpanded template line number and therefore intervened before the old hash check. The old launcher rejected that earlier change, so the intended RED assertion failed. This was a test injection error. It was corrected to find the line in the actual rendered old launcher, then the nine cases above completed. The initial `local-fixtures-zyzdvw7o/` and selected failure evidence under `evidence/harness-correction/` are retained; they are not silently reported as a completed regression run.

An intermediate successful nine-case run, `local-fixtures-0lupfz9o/`, labeled the implicit subprocess working directory as this source directory although the process inherited `/workspace`. Its absolute `git -C` targets and results were correct. The logging field was corrected to `Path.cwd()`, script hashes were added, and all nine cases were rerun for the final evidence above. The intermediate raw directory is retained locally; it is not the final command receipt.

## What is proved and what is not

The old check-then-execute branch is removed. No new or old original-hook bytes are executed by the repaired launcher. Observed active hooks, configuration changes, and directory identity/permission changes cause rejection. A generic fd-execution substitute was rejected because script interpreters and relative `$0`/`__file__` behavior cannot be claimed faithful at an invented fd path.

This does **not** lock configuration or directories against arbitrary future writes by the same UID. An original hook created after the final inspection, a same-UID edit of the launcher, or a concurrent configuration change after its last read is outside that guarantee. Permission observations are not OS-level exclusion; neither the launcher nor these tests prove total local race safety. Immediately before any separately authorized operation, recheck the actual effective hooksPath, absence of active original hooks, and permissions. If they differ, stop and return to parent review. Do not chmod/chown the real repository or change global security settings without specific authorization.

The accepted remote concurrency boundary is separate: the old OID is checked against the transport advertisement; the server checks the advertised old OID when processing the request, and `--atomic` makes the ref update batch all-or-nothing. The retained v7 before/after-advertisement tests are historical evidence for that mechanism; this repair does not relabel them as new production execution. Git documents the [pre-push arguments and four input fields](https://git-scm.com/docs/githooks#_pre_push), [hooksPath selection](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corehooksPath), [atomic pushes](https://git-scm.com/docs/git-push#Documentation/git-push.txt---atomic), and the [advertised old-ID ref-update check](https://git-scm.com/docs/gitprotocol-pack).

The guard cannot protect a GitHub automatic source-branch deletion, web UI deletion, API action, or another client that never invokes this launcher. It does not itself prove durable backup availability or authorize merging/deleting any branch. Those remain parent gates. No force option, lease, protection change, or production cleanup command was used.

## Review and future use boundary

`review-files.txt` is an explicit text-only copy whitelist, and `SHA256SUMS` pins those files. Do not recursively copy fixture bare repositories or object packs into public evidence. The copied sources and fixture hooks are evidence, not installation instructions.

`prepare.py --repository <repo> --manifest <reviewed-manifest> --output <new-temporary-directory>` renders a new **disabled** launcher after inspecting the real repository. It does not install it or change Git configuration. Any future activation requires the already-required explicit parent approval, a fresh actual environment check, a newly approved remaining-ref manifest, and newly reviewed exact hashes. The only supported future transport form remains a command-scoped hooksPath override on an ordinary `git push --atomic origin --delete ...`, with no force or lease options. This document does not authorize running that command now.

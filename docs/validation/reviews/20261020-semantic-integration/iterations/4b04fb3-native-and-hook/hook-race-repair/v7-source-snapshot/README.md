# Reviewed cleanup guard preparation

Status: **PREPARATION ONLY / PRODUCTION NOT APPROVED**. No GitHub ref, integration file, repository hook/configuration, or production branch was changed. The parent has approved only the direction of one ordinary #71 merge; neither merge nor deletion is authorized yet. All actual pushes below targeted local fixture bare repositories.

## Files to review and copy

The public-copy text set is this README, `guard.py`, `prepare.py`, `pre-push.template.py`, `exercise_local.py`, the two manifest templates, `candidate-branches-4b04fb3.NOT-APPROVED.tsv`, `fixture-object-scope.json`, `reviewed-sha256.json`, `SHA256SUMS`, and `evidence/`. Do not copy the `local-fixtures-*` bare repositories into the public evidence attachment. Their text evidence has been copied into `evidence/`.

- `guard.py` validates the actual four-field advertised batch. It never updates refs.
- `pre-push.template.py` first executes the original pre-push hook with identical stdin bytes/arguments/cwd and inherited environment, propagating its failure. Then it checks fixed manifest/guard hashes and executes the exact hashed guard bytes. Changed original hooks abort preparation use; they are not silently bypassed.
- `prepare.py` renders a fresh external hook directory. It never writes repository configuration or pushes. It forwards other original hook entries to their original paths, because a temporary `core.hooksPath` affects all hook names. The original hook directory must be explicitly inspected, including any existing `core.hooksPath` setting.
- `approved-manifest.template.json` is an unfilled, disabled template. `candidate-manifest-4b04fb3.NOT-APPROVED.json` and the TSV contain eight concrete non-main review candidates with exact backup heads/refs. The integration entry is **4b04fb3168cdc9bf8f54d7755db94fd3677eebb6**, not the older 1b2 fixture value. They remain disabled through `approval: TEMPLATE_NOT_APPROVED`. `refs/heads/main` and every tag are prohibited.

The original seven non-main heads were copied from the parent's existing backup manifest; the integration entry comes from `integration-backup-4b04fb3.json`. The parent reports 22 durable recovery tags and matching nine branches. This preparation did not repeat remote backup validation. After an authorized merge, remove already auto-deleted branches from the proposed batch only through a fresh parent review, and review any new head against its own durable backup. Do not silently replace an expected OID with a new advertised value. A later source change requires a new candidate manifest and approval.

## Mechanism and official references

Git calls pre-push with remote name/location and stdin `<local-ref> <local-object-name> <remote-ref> <remote-object-name>`. Deletion uses `(delete)` and an all-zero local object ID; a nonzero hook exit aborts the push. The guard requires deletion-only rows, the complete approved ref set, non-main branch namespace, exact approved advertised OIDs, and matching remote name/URL. It also refuses multiple effective push URLs: atomicity is per transport connection. [Official githooks manual](https://git-scm.com/docs/githooks)

The push protocol sends deletion as `old-id zero-id ref`; the receiving server checks that each ref still has that old ID. This closes the window after advertisement. [Official pack protocol](https://git-scm.com/docs/gitprotocol-pack)

`--atomic` makes the receiving update all-or-nothing; if the server does not support atomic pushes, Git fails. There is no fallback to a non-atomic attempt. No force or lease option is used. [Official git-push manual](https://git-scm.com/docs/git-push)

| Change timing | Protection | Actual local result |
| --- | --- | --- |
| Before advertisement | Fourth stdin field differs from approved head | Hook rejects; both branches remain. |
| After advertisement, before server transaction | Server old-ID comparison plus `--atomic` | Server rejects both deletions; changed branch and unchanged peer remain. |
| No change | Exact approved batch | Both approved branches deleted; main, control tag and unapproved branch remain. |

## Actual local transport evidence

`evidence/results.json` records **16/16 passing scenarios**, Git **2.52.0**, per-case commands/statuses and final refs. `evidence/commands.jsonl` is the actual command/stdout/stderr journal. Each `evidence/cases/*/result.json` records complete before/after refs. Packet traces and original-hook stdin/arguments are retained alongside the cases.

Only existing sample tips `89d806c8aec961a7991eb3e6d6e2b1370aed8791` and already backed-up `1b2c07da2cb2b0c89c64ca546148fe5d68d75a97` were used. A writer changed one fixture ref from 1b2 to 89; it did not create an unbacked commit. `fixture-object-scope.json` confirms all **1314 commit objects** in the seed are exactly in those tips' ancestor closure; **zero commits were created**. Bare clones share that immutable local seed. `GIT_ALLOW_PROTOCOL=file` prevents the exercise from contacting network transports.

The matching two-ref deletion succeeded. Change-before-advertisement and change-after-advertisement scenarios both preserved the pair of refs. The latter's fixture-only hook wrapper waits for the real guard to approve, changes one receiver ref with local `update-ref`, then returns to the real push. The packet trace contains the original advertised old ID and deletion command. The receiver reported:

```text
cannot lock ref 'refs/heads/fixture/a': is at 89d806c8aec961a7991eb3e6d6e2b1370aed8791 but expected 1b2c07da2cb2b0c89c64ca546148fe5d68d75a97
fixture/a (atomic transaction failed)
fixture/b (atomic transaction failed)
```

The remaining scenarios rejected main, tag, non-deletion, unapproved ref, changed manifest bytes, changed guard bytes, existing-hook refusal, changed original hook, wrong URL, wrong remote name, multiple push URLs, incomplete batch and missing server atomic support. The original hook received byte-identical stdin and both arguments; its direct launcher exit remained **37**, while Git itself reported push failure. Another original hook was actually forwarded and invoked. Rejections assert their specific reason, not merely an arbitrary nonzero exit.

Commands actually executed:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 exercise_local.py
PYTHONDONTWRITEBYTECODE=1 python3 exercise_local.py --seed local-fixtures-elg0t97l/seed.git
```

The second run added explicit rejection-reason assertions, capped pack threads and reused the existing local seed. Its retained result is `local-fixtures-wum6z4xv`; copied public evidence is under `evidence/`. No full product test, CI dispatch or production push was started.

## Future parent-controlled operation

The already rendered `production-template-hooks/pre-push` is reviewable external text, not an installed hook. It pins the current **NOT-APPROVED** manifest and must refuse production use. Its absolute paths are intentionally local; copying the text elsewhere requires rendering a new launcher and reviewing its new hash. Never edit a pinned manifest in place and keep using the old launcher.

After explicit parent deletion approval and the existing merge/recovery/acceptance prerequisites, create a fresh immutable approved manifest for only the remaining unchanged branches, render a new temporary hooks directory against the repository's actual original hook directory, and review all resulting hashes. Before any push, verify the actual launcher exists, is executable and matches the reviewed hash, and verify its fixed manifest/guard bytes. Git silently ignores a missing hook directory, so a placeholder `core.hooksPath` is not protection.

The reviewed operation shape is ordinary `git -c core.hooksPath=<verified temporary directory> push --atomic origin --delete <explicit full approved refs>`. It uses no force, lease, wildcard, mirror, no-verify, or persistent configuration change. The exact ref arguments must match the whole approved manifest. This README does not execute that operation or grant its approval. If any check fails, stop, record actual remote state and return to parent review; do not retry with relaxed checks or refresh expected heads automatically.

## Limits

This is a local concurrency preparation proof, not a successful GitHub cleanup. No claim is made about production authentication, GitHub atomic capability, ruleset acceptance, final CI, or production deletion. Server protection may reject an otherwise valid batch, which must remain an error. The hook cannot inspect whether the command included `--atomic`, so the parent must pin and review that exact command. It is not a defense against an operator who bypasses/replaces local hooks or edits reviewed inputs/commands.

GitHub's automatic branch deletion on PR merge does not pass through this client hook. The guard applies only to an explicitly approved client deletion of remaining refs. It does not create a global writer lock, detect an away-and-back OID history, stop a later writer from recreating a branch, or prove recovery from a backup-ref string. A network or post-acceptance failure may leave an uncertain result; observe actual refs before any further action.

Production remains **NOT APPROVED AND NOT EXECUTED**. Scripts, candidate manifest/TSV and evidence are frozen for independent parent review. Exact hashes are in `reviewed-sha256.json` and `SHA256SUMS`.

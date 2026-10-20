# Semantic integration review evidence — 2026-10-20

English | [简体中文](README.md)

This dated review attachment records the evidence submitted for [PR #71](https://github.com/21Z121Z1/XDRemux/pull/71). It is retained through a dedicated Git tag so that publishing the attachment does not move the PR head or cancel its running checks. It is an audit snapshot, not current branch instructions or a declaration that convergence is complete. The parent explicitly requested public recovery evidence; this directory contains public fixture identities and textual logs, no private media or credentials.

Product source under review is `f9a419aa4eeb33214e2dfac2205719e6cb97d598`, tree `02cda5ccb3daead965e35aba121bcc624a4cd63a`. The recorded default branch is `89d806c8aec961a7991eb3e6d6e2b1370aed8791`. The attachment commit is documentation/evidence only and has a different SHA. Evidence for the product source must not be relabeled as verification of that attachment or a later product head.

## Inspect the original evidence

| Evidence | Inspectable file | Scope |
| --- | --- | --- |
| All nine durable backup refs, exact commits and trees | [Recovery manifest](recovery/manifest.json) | Eight original heads plus integration. |
| Original manifests, byte-for-byte | [Original branches](recovery/original-branches.json), [integration](recovery/integration-f9a419a.json) | Retains the originally reported SHA-256 identities. |
| Remote restoration and fsck | [Original restore log](recovery/original-restore.log), [integration restore log](recovery/integration-restore.log) | Fetched from GitHub into a fresh bare repository, compared exact commit/tree, fsck exit 0. The unborn bare HEAD notice is expected. |
| Nine-check committed-HEAD receipt | [Full receipt](verification/portable-receipt-f9a419a.json), [plan](verification/portable-plan.json), [run/verify log](verification/portable-gate-f9a419a.log) | All checks mandatory and passed; clean tracked tree and unchanged head verified. Linux portable scope only. |
| Complete portable suites | [Rust log](verification/rust-workspace-f9a419a.log), [Python log](verification/python-f9a419a.log) | 318 Rust passed, 0 failed, 1 explicit vendor-matrix ignore; 219 Python passed. macOS cfg-gated zero-test binaries do not count as Apple execution. |
| Actual public CLI conversion | [Fourteen-case manifest and logs](verification/real-cli-matrix-f9a419a/manifest.json) | 14 original fixtures, both members of every pair independently validated. Output hashes retained; run-owned scratch media removed. |
| GitHub progress snapshot | [Checks captured at 19:27:56 UTC](verification/github-checks-f9a419a-20261020T192756Z.json) | Immutable historical progress, not a live final status claim. Follow each linked run for completion. |
| Corrections, prototype and enforcement | [Evidence map](evidence-map.en.md) | Risk, actual source, resulting invariant, benefit, check, and remaining evidence limit. |
| Merge and cleanup proposal | [Merge plan](merge-plan.en.md) | Exact observed original heads and stop conditions. No merge/delete permission is conveyed. |

`SHA256SUMS` identifies every attachment file except itself. Verify from this directory with `sha256sum -c SHA256SUMS` or `shasum -a 256 -c SHA256SUMS`. The original eight-head manifest hash is `908b9c251b83c9f37ce54103530e944d9d8d1461fc819c8c946621bcced5bd69`; the integration manifest hash is `bcd02d0fe8d39e7864012c517e975e748269d7ff4ea47e2c31229d9056ac198e`; the full nine-check receipt hash is `9be89f676ae7974105050c0570b17b0efe82072dd3ab2000326713b131ccbbf7`.

The plan records commands and their original absolute paths as executed. The recipe source is retained as [text](verification/verify-current-cli.py.txt), not a new executable runner. For a fresh run, read the current project verification skill and use the existing completion gate with current paths, base and source identity. The old receipt is deliberately invalid for a later HEAD.

## Native and device limits

Existing runs for the product SHA are [product/App/ImageIO](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011626), [required exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011433), [research/System HEIF](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011596), [performance](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011670), [portability](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011556), and [real ProXDR](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011714). At the recorded snapshot, CodeQL, real ProXDR, portable research, tone response, Linux portability and macOS portability passed. Product, exact-head, Windows and remaining native research/performance were running or queued. No duplicate run was started for this attachment.

#65's historical author report says the generated files failed actual iPhone Photos Styles activation. Structural/native readback and a tone sweep do not prove Tone/Color activation, edit/save/reopen or Gallery behavior. Diagnostic artifacts are named accordingly. The integration retains v15 and #70's real per-image native Texture path; no fabricated capture parameters or version-only activation claim was promoted. Performance thresholds and baselines were not relaxed. No Metal fault, hardware OOM or device behavior was simulated and represented as a real native result.

## Selected pstack methods

Official pstack is pinned to `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`; the six project skills retain original MIT license plus path/blob/SHA-256 provenance. Actual `sharpen` and `in-your-own-words` methods were used to define observable media preservation and evidence classes; `correct` maps demonstrated repeated errors to existing machine checks; `architect` compared existing owners and caller shapes before edits; `create-verification-skill` produced and exercised a real CLI feature recipe. TDD, engineering principles, review and prose cleanup were used for the actual fixes and this evidence pass. The verification recipe's initial existing-binary demonstration and subsequent committed-HEAD gate are distinct records.

No Cursor runtime, bootstrap/Bun setup, model routing, automatic external chat/ticket, shipping/autopilot or cleanup authority was imported. Recall/Slack history was unavailable and was not claimed. No separate framework, policy engine or second result ledger was added to the product. The pinned official DeepSeek `dsh-find-simplifications` was also read; concrete consumer, failure, ownership and recoverability obligations were retained.

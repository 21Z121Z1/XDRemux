# Merge and recovery proposal for parent review

English | [简体中文](merge-plan.md)

No PR merge, branch deletion, protection change, force push, upstream write or release is authorized by this document. The parent has not yet approved convergence. All values below are observed snapshot identities; fetch again before acting. Product review surface is PR71 at `f9a419aa4eeb33214e2dfac2205719e6cb97d598`.

## Exact observed PR heads and order

| Order | PR | Source branch | Observed exact head | Required semantic preparation before a normal merge |
| --- | --- | --- | --- | --- |
| 1 | #58 | codex/liblivephoto-consumer-20261004 | 0688ea7d6270df52cc5968e2c36dd016f13bd31b | Refresh current main and retain pinned LibLivePhoto consumer; apply primary-video/sample, exact-time, unmapped-HDR and applicable app/batch publication repairs. |
| 2 | #69 | cursor/portrait-heic-orientation-4765 | 7952d0b19c6b47738cc0722acae57465a243186f | Refresh resulting main; retain coded/display geometry with complete mirror/order handling and ImageIO product evidence. |
| 3 | #65 | feat/styles-hdr-first-linear-thumbnail-20261015 | bc4d54753f18905dbe73f595a06a6eaf21aaa014 | Refresh resulting main; preserve reconstruct-before-reduction and per-fixture diagnostics, v15 and #70 native Texture. Exclude unsupported v16-only promotion. |
| 4 | #68 | cursor/full-res-linear-hdr-e174 | 1f29e3cdc1b8fd49d56718ceecf96b8de80ec817 | Refresh resulting main; retain full reconstruction/true area reduction, reciprocal gamma, RGB16/Main10, submitted system pair and fixed scaffold graph; close actual native/performance failures. |
| 5 | #71 | integrate/semantic-review-20261020 | f9a419aa4eeb33214e2dfac2205719e6cb97d598 | Reconcile remaining integration-only changes and ancestry after original PR handling; validate the resulting final head. The current combined green checks cannot make older failing original heads green. |

#85 returned API 404. It is not an open PR to operate on. #65 and #58 are currently draft; ready-for-review is permitted only once their concrete accepted candidate exists.

These exact observed original heads are **not merge candidates** merely because the integration is tested. Preparing a source head changes its SHA. Strict up-to-date main also means later final heads cannot be honestly predicted before preceding server merge commits exist. For each stage, record the actual new SHA, its new immutable backup ref and recovery proof, exact-head/native checks and current base. Return that concrete stage for parent review if it changes this proposal; never substitute an old SHA, a PR merge ref or an untested conflict resolution.

## Execution gate for every stage

1. Fetch only `21Z121Z1/XDRemux`. Compare remote main/source with the recorded values and inspect all worktrees for uncommitted or concurrent writes. Stop on movement until reconciled; never overwrite another writer.
2. Prepare the smallest semantically complete candidate in an isolated worktree. Keep the original source head as an ancestor for a normal fast-forward source push. Resolve conflicts by current behavior, not by choosing an entire stale tree. Run its applicable regression/functional/native checks and review the diff.
3. Create a **new** persistent tag for the exact candidate, push only that tag, fetch it into a fresh recovery repository, compare commit/tree, run fsck and retain the log. Never move the original backup tag. This must precede any merge that can auto-delete its source.
4. Publish the candidate to its PR source only after rechecking its old remote head. Derive the newly published exact SHA again; inspect existing runs and do not duplicate a running check. Wait for all required and applicable checks on that exact SHA, including strict `exact-head`, CodeQL, native product and unchanged performance limits.
5. Re-read default-branch rules, review threads, main/source heads, auto-delete setting and writer state. Resolve review threads. Only after parent release of the merge gate perform a normal GitHub merge with `expected_head_sha` equal to the accepted exact candidate. Do not bypass or alter rules. Verify returned merge SHA and actual remote state before proceeding.
6. Reconcile the next PR against that actual new main. A changed head invalidates the earlier receipt. Keep evidence for old and new heads separate.

Active ruleset 21932863 requires PRs, strict `exact-head`, resolved threads and CodeQL high/errors, prohibits default-branch deletion/non-fast-forward, and offers no bypass. Auto-delete-on-merge is enabled. No setting was changed by this task. Legacy branch-protection administration reads return connector 403; active ruleset and repository/branch metadata were read through the authorized read-only API. No unavailable administrative access is assumed.

## Cleanup candidates after final review

The [nine-row recovery manifest](recovery/manifest.json) is authoritative for original backups. Original seven non-main branches plus the integration branch are cleanup candidates only after reviewed semantic disposition, final-head backup and parent release. Keep main and every backup tag. Re-list branches immediately before deletion and compare the exact candidate head; new commits require a new backup and review. No unrecorded branch is covered by this proposal.

| Residual branch | Exact head | Unique content and proposed disposition |
| --- | --- | --- |
| feat/rust-cli-distribution-i18n-20260924 | c6d307af662ed516f6e3994c039513359cb98610 | Six unique commits contain real bilingual CLI and macOS packaging. **Parent choice remains pending:** selective port or immutable backup plus explicit deferral. Wholesale import would remove current native targets and lacks relocated unpacked conversion proof. It is not already merged/equivalent. |
| research/texture-style | f8fcf2067c506e87665e9ea113324db1a57d4cf7 | Retired fixed/aliased carrier generation is semantically superseded, not ancestry-equivalent. Main retains original contracts; native-standard matrix SHA256 is `2bb45a9273a270802da6148b21431666a03dd8d0296915532e396bf197277d4d`. Keep backup, do not restore fabricated capture/resource hooks. |
| research/texture-native-20261019 | 18340989590d451f5676924feb03c70baa503e1a | Additional opaque inspect/preserve APIs have diagnostic/research consumers, not a demonstrated missing product consumer. Preserve unique 147 JSON and 21 native failure logs in backup. Do not call failed native evidence device acceptance. |

Restore a branch without touching main or rewriting history:

```sh
git fetch origin refs/tags/backup/semantic-review-20261020/cursor/full-res-linear-hdr-e174
git switch -c recovered-hdr FETCH_HEAD
```

No original PR was merged or closed and no branch was deleted when this attachment was prepared. Final main SHA and one-branch status remain pending actual merges, native validation and parent review.

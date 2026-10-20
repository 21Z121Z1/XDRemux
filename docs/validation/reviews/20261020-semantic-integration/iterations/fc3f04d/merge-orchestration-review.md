# Merge orchestration review

Public attachment: the skill link is adapted to the exact reviewed commit. Original external report SHA256: `9137177326be8d4fcccb7bf21654c293acdb43689d0e964f5cafadd276f1a3f7`.

Read-only independent review, 2026-10-20. Repository: `21Z121Z1/XDRemux` only. GitHub state was captured at `2026-10-20T21:24:01Z`; all observations are conditional on those refs/rules remaining unchanged. The review recommends an alternative for parent consideration; **the current sequential plan and closed parent merge/deletion gate remain in force until the parent explicitly approves a change.**

## Recommendation

Once the final #71 source has passed every existing parent acceptance requirement, a single ordinary merge of #71 is a smaller coherent candidate than recreating five independently accepted source states. It preserves all four original PR tips in main's ancestry, and GitHub documents the resulting original-PR status as an indirect merge. This is not equivalent to passing those original heads' individual gates. Approval must expressly replace the earlier staged merge requirement with final integrated acceptance, retain the original review/defect dispositions, and accept indirect `merged` status for #58/#69/#65/#68. Without that explicit parent decision, retain the staged plan.

This review is authorized by the parent's specific independent-review delegation. `AGENTS.md` directs contested designs to the adapted [pstack review skill](https://github.com/21Z121Z1/XDRemux/blob/fc3f04db604e8d7f72a9dbe1305c5a9829472327/.agents/skills/pstack-review/SKILL.md); it permits independent authorized reviewers, requires producer/consumer evidence and lead judgment, and does not authorize merges or external changes. The previously user-selected DeepSeek simplification skill distinguishes proposals from implementation authority and asks for consumer evidence and rejected alternatives. Those scopes fit this read-only comparison. No further agents were spawned. This review performed no product edit, staging, commit, branch/ref advance, merge, close, delete, CI dispatch, protection/billing change, upstream read/write, or backup revalidation. Only these external audit files were written.

## Actual identity and ancestry

Fresh connector reads confirm main is still `89d806c8aec961a7991eb3e6d6e2b1370aed8791`; #71 is open/draft at `fc3f04db604e8d7f72a9dbe1305c5a9829472327`, tree `30658a3a63fb3ebb1907b2555f9b0678e9d98711`. All original PRs are still open/unmerged, same-repository PRs targeting main, at the heads below. #58 and #65 are drafts. Every queried review-thread list and review-submission list for all five PRs is empty. These are current observations, not permission or a guarantee that later comments/refs will stay unchanged.

`git merge-base --is-ancestor` returns 0 for main and each original tip into #71. It returns nonzero for each original tip into current main. `git rev-list ORIGINAL --not CANDIDATE --count` returns 0 for each PR: no original commit is missing. The existing integration commits are real two-parent merges, with each original tip as the second parent:

| PR | Exact original tip | Introducing merge in #71 | Commits missing from #71 |
| --- | --- | --- | ---: |
| #58 | `0688ea7d6270df52cc5968e2c36dd016f13bd31b` | `96bb9391ecf34b59a2ca0a3554814acc843eb737` | 0 |
| #69 | `7952d0b19c6b47738cc0722acae57465a243186f` | `f0292a994de5ff265d1d0aa19426bf3aa1ff1da9` | 0 |
| #65 | `bc4d54753f18905dbe73f595a06a6eaf21aaa014` | `6f637aeae5abc8df403a06be383c54021309aa7f` | 0 |
| #68 | `1f29e3cdc1b8fd49d56718ceecf96b8de80ec817` | `083b9d35d4f88ef3406ff44c294edded2aea996f` | 0 |

The candidate differs from main in 163 paths, with 14959 additions / 8244 deletions. Thus preserving ancestry is not a claim that every original patch remains behaviorally identical. Later commits preserve, correct, or supersede particular behavior; their final consumers require the #71 acceptance evidence. Patch IDs alone would not establish this ancestry or final semantic acceptance.

A read of GitHub's current test merge `eaf61e264a9de9f512439eebea9d8b3739bd2035` confirms parents `[89d806c8…, fc3f04d…]` and exactly the candidate tree `30658a3a…`. This is the server's present test merge, **not an accepted future merge SHA**. A final ordinary merge creates its own commit; the actual resulting parents/tree must be read after an approved merge.

Reproducible evidence: [ancestry/parents/counts](merge-orchestration-ancestry.json), [fresh PR/main/ruleset/test-merge snapshot](merge-orchestration-github.json), [review submissions](merge-orchestration-reviews.json), and [actual patch/intermediate evidence](merge-orchestration-patch-evidence.txt). No new object/merge was synthesized to obtain these results.

## Will GitHub mark the original PRs merged?

Yes, the proposed ordinary merge satisfies GitHub's documented indirect-merge condition: their original head commits would become reachable from their base, main, through #71. GitHub distinguishes this from manual closing; it also explicitly states indirect `merged` status can occur despite unsatisfied protections on the original PR. The expected result must be observed through each PR's `merged`/`merged_at` state afterward; Git reachability does not prove GitHub's asynchronous bookkeeping has already completed. [Official pull request merges documentation](https://docs.github.com/en/pull-requests/reference/pull-request-merges#indirect-merges)

Use the ordinary merge method to preserve those original commit identities. Squashing or GitHub rebasing #71 rewrites the incorporated history and does not establish the same reachability; this alternative cannot rely on patch equality to claim indirect merges. No manual close/retarget/source-branch update is needed for the proposed result. [Official merge strategies](https://docs.github.com/en/pull-requests/reference/pull-request-merges)

This is the central limitation: GitHub can show the four PRs as merged without requiring four independent successful old-head checks or resolving hypothetical new original-PR threads. The parent must accept the integration as the reviewed acceptance surface. Preserve factual statements such as “original commits integrated through #71; final source H passed checks X,” and never replace them with “all four original heads passed.”

## Existing enforcement on the actual #71 source

Fresh API reads found one active repository ruleset, ID `21932863`, targeting the default branch:

- Deletion and non-fast-forward updates are prohibited.
- A pull request is required; ordinary merge is an allowed method.
- `exact-head` is required with `strict_required_status_checks_policy: true`.
- Review-thread resolution is required. Required approval count is zero; stale-review dismissal, code-owner review, and last-push approval are currently false. `require_extra_approval_for_unattributed_changes` is true; do not infer a new approval rule from this field or change it.
- CodeQL blocks security alerts `high_or_higher` and ordinary alerts `errors`.
- `bypass_actors` is empty and `current_user_can_bypass` is `never`.

The connector rejected the effective `/rules/branches/main` URL as an unsupported fetch endpoint (HTTP 400 / `INVALID_ARGUMENT`). The allowed `/rulesets` collection and exact ruleset GET succeeded, as did the main branch read. This was an endpoint-schema limitation, not a denied merge attempt, and no rule mutation or forbidden-route retry occurred. The report uses those successful authoritative reads, rather than interpreting the classic protection subobject's disabled status as “main is unprotected.”

Strict mode still requires the merging branch to contain the then-current base. Today main is an ancestor of H, so the observed graph meets that condition; if main or H changes, stop and re-evaluate the new source and all required evidence. GitHub documents extra builds when strict branches need refreshing, and requires conversation resolution when configured. [Ruleset status/review requirements](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets), [protected-branch behavior](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)

`completion-gate.yml` selects `github.event.pull_request.head.sha`, fetches full history, and binds the receipt to that source. Product/App, portability/package, fixtures, research and performance workflows also explicitly check out the PR source; package/product evidence records its identity. Parent acceptance must read their actual run SHA, checkout/source identity, outcomes and artifacts for final H. Ruleset-required `exact-head` and CodeQL do not replace the parent's additional native/SystemHEIF/performance/package acceptance. An interleaved performance comparison remains diagnostic; it cannot waive an absolute budget failure.

For CodeQL, inspect its actual analyzed revision and the current PR association. GitHub recommends PR merge-commit analysis, so a CodeQL run may legitimately analyze the test merge rather than raw H. In that case bind it to the current base/H parents and equal tree shown above; do not relabel a previous test merge or another PR's scan as current. [Official CodeQL workflow guidance](https://docs.github.com/en/code-security/reference/code-scanning/workflow-configuration-options#scanning-pull-requests)

The final authorized merge should be constrained to the reviewed expected head H and ordinary merge method, and must be rejected if that head moved. GitHub's documented merge API supports the expected head SHA and returns conflict on mismatch. This is a description for parent review, not an executed command. Read #71's current draft/mergeability, rules, reviews/threads, base/main, old heads and check results immediately before it. Draft #71 is presently not ready to merge. [Official merge API](https://docs.github.com/en/rest/pulls/pulls#merge-a-pull-request)

The repository has `delete_branch_on_merge: true`. An approved merge therefore has automatic branch-cleanup consequences even without an explicit delete request. Do not promise original/integration branches will remain, or promise every indirect branch will be deleted: observe GitHub's result and apply the parent's existing verified-recovery prerequisite before authorizing the operation. The parent reports 19 backups/restores complete; this review did not repeat or extend that work. [Official automatic branch deletion documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-the-automatic-deletion-of-branches)

## Comparison with the existing staged plan

| Aspect | Existing #58 → #69 → #65 → #68 → #71 plan | Conditional single ordinary #71 merge |
| --- | --- | --- |
| Acceptance identities | Five refreshed, separately accepted heads against each then-current main. The original stale greens are insufficient. | One final integrated H; preserve original heads' actual outcomes separately. |
| Main updates | Five merge points with four intermediate main trees. | One main transition from current main to the reviewed final content. |
| CI work | At least four additional distinct head-validation cycles relative to a final integrated acceptance; each main update also creates a push-validation opportunity. Exact job counts depend on path filters. | One final-head acceptance cycle plus normal post-merge main checks; future source/base changes still require fresh evidence. |
| Strict ancestry | Each server merge advances main. Later sources, including #71, must incorporate those actual new main commits and requalify. Tree similarity alone cannot make H contain new server merges. | Main is already an ancestor of H while it remains unchanged. |
| Repairs and conflicts | Applicable repairs must enter the earliest safe stage, flow through later main updates, and be reviewed/tested in each resulting composition. | Existing integration already contains the reviewed composition and subsequent fixes; its final checks remain mandatory. |
| Original PR status | Four explicit direct PR merges, then residual integration. | Four documented indirect merges through #71; no claim of original-head green checks. |
| Review/history tradeoff | Stronger direct per-PR audit and smaller first-parent merge steps; easier isolated attribution when stages are independently valid. | Fewer transient states; original commits/local merge boundaries remain inspectable, but main's first-parent merge and revert unit is the larger integration. |

These are orchestration counts, not estimated runtime or budget savings. No CI was started here and no threshold/runner setting is proposed for change.

Concrete intermediate-state obligations, from source rather than assumption:

1. The #65 integration merge `6f637ae` still reconstructs reduced HDR with four taps (`apple_styles.rs`, `let taps = if reduced { 4 } else { 1 }` and the stratified 2x2 comment). #68 and later integration supply full-resolution reconstruction/true area behavior and RGB16/current scene bindings. A staged #65 main would therefore deliberately precede that final contract. This can be a valid staged delivery, but cannot be described as already delivering the final full-resolution result.
2. The four local merge stages precede `9647b29` (complete coded transforms, mirror association order, actual submitted HDR scene), `4453e3d` (reject incomplete native GPU receipts and verify edit state before reuse), the scaffold/alpha repairs, and final publication changes. None of those historical stage trees is automatically accepted by a green final tree. The existing staged plan correctly requires carrying applicable repairs forward; the alternative removes the need to publish those separately reconstructed compositions.
3. The important latest media fix `5748768` addresses a main-existing publisher. `git rev-parse 89d806c8:crates/xdremux-motion-photo/src/publication.rs` and `5748768^:...` both return **`c7b3177096b6cdcf60529b52d48570ac4124767b`**, and the intervening file diff is empty. The removed filename-cleanup/replacing-rename behavior was not introduced by the four feature PRs. Staged acceptance needs this fix at the earliest stage that promises media safety; it is not evidence that a feature PR caused the issue, nor a reason to merge an unsafe intermediate.

The strongest reason to retain the current plan is the parent's explicit staged-acceptance requirement and direct review/audit granularity. The strongest reason to approve the alternative is that the final intertwined media/native fixes already form one reviewable composition; fewer distinct publication states and source-branch writes can preserve the same actual product acceptance without pretending old heads passed.

## Parent decision and acceptance criteria

**Recommend “consider/approve conditionally,” not “merge now.”** Parent approval must explicitly choose the indirect-merge alternative in place of the existing staged sequence. It should name final H, ordinary merge, the unchanged gate, all four incorporated original tips, and the automatic-deletion consequence. If that change is not approved, the current sequence remains the only proposed execution path.

Before any approved merge, retain the existing recovery proof and review dispositions, require all final-source exact-head/native/App/SystemHEIF/performance/CodeQL/package/fixture obligations selected by the parent to pass, and reread current rules/threads/refs. Pending or failing final-source evidence remains blocking under either plan. This report neither predicts the queued jobs' outcome nor uses earlier heads' passes to satisfy it.

After an approved ordinary merge, record the actual merge commit/main ref, parents and tree; confirm H and all four original tips are reachable; observe each original PR's actual `merged` state; and preserve new-main CI/source evidence separately from H. Do not manually close a still-open PR or delete a branch to manufacture the expected result. If GitHub bookkeeping disagrees or a source moved, report the discrepancy for parent review.

Rejected alternatives: squash/rebase #71 does not provide the required original-commit reachability; advancing all four original branches to H adds unnecessary writes/scope changes and can interact with same-head review blocking; manual closing supplies no merged evidence; relaxing strict checks, changing budgets, dropping native/performance gates, or relying on an old passing head changes acceptance rather than simplifying orchestration. None was executed.

## Review result and remaining limits

- **Act on, P1 process risk:** indirect `merged` status does not enforce each original PR's individual protection/acceptance. Explicitly amend the parent plan or keep the sequential plan. This is documented GitHub behavior, not a claim of an exploitable new code defect.
- **Act on, P2 evidence risk:** distinguish source H, current test merge, actual future main merge and original tips. Every changed base/head invalidates assumptions about ancestry/evidence binding.
- **Noted:** all queried review threads/reviews are currently empty, but #71 remains draft and final checks are pending/in progress per the parent. Empty lists do not authorize a merge.
- **Noted:** verified backup preparation is a parent-provided prerequisite already complete; it was not rerun. Automatic cleanup settings remain unchanged.

The worktree remained clean at review end. No merge/delete/source-advance attempt was made, so none has a pending or uncertain mutation outcome. The only blocked read was the unsupported effective-rules fetch URL; successful ruleset/main reads provided the needed evidence. Final plan choice and every subsequent mutation remain unapproved in this task.

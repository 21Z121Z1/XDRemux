---
name: pstack-workflow
description: Coordinate a scoped XDRemux investigation, bug fix, feature, or refactor using pstack's reproduce, design, implement, verify, and review sequence. Use when the user requests pstack or when a substantial engineering task needs this workflow. Skip ceremonial planning for a small mechanical edit.
---

# XDRemux engineering workflow

Read [AGENTS.md](../../../AGENTS.md), the relevant owner, and nearby consumer tests before choosing a change. Treat this skill as a project workflow. It grants no additional authority to send messages, update tickets, publish, open or merge a PR, commit, push, delete branches, install tools, or change billing or protection settings. Follow the user's current scope and the active session's permission rules.

## Choose the work

1. State the user-visible result, current behavior, affected producer and consumer, and the evidence that will distinguish success from failure.
2. Read the matching [playbook](references/playbooks.md). Keep investigations read-only. Keep refactors behavior-preserving. Name any intended behavior change explicitly.
3. Name the data shape, canonical owner, and boundary before changing a cross-function contract. Compare a small number of alternatives when the tradeoff matters. Do not create an abstraction for a single mechanical edit.
4. Read [engineering principles](../pstack-engineering-principles/SKILL.md). For a bug with a cheap executable reproduction, read [TDD](../pstack-tdd/SKILL.md).
5. For multi-step work, keep a short plan with the blocking evidence, independent work, shared files or state, and smallest useful check for each unit. A single owner is sufficient when splitting work adds coordination without improving coverage.

## Implement and inspect

Make the smallest coherent change supported by the evidence. Preserve existing caller contracts, public fixture bytes and provenance, source facts, and failure-safe publication. Re-ground after another worker changes shared files. Inspect the actual diff and its consumers before accepting delegated work.

Use the session's available tools. Delegate only when the task or applicable instructions authorize it and the work has a useful independent scope. Give each worker its target files, intended behavior, known constraints, and required evidence. Serialize shared writes. Inherit the active session's model and settings unless that session explicitly specifies otherwise. Do not select a new model, reasoning budget, or service tier from this skill.

## Verify and review

Follow the [verification checklist](references/verification.md) and [XDRemux feature recipes](../xdremux-verify/SKILL.md). Run the affected user entry point or real artifact and the smallest complete repository checks. A compile, structural parse, native framework readback, and real-device edit are different claims. An unavailable host or failed check remains unavailable or failed.

Use [review](../pstack-review/SKILL.md) for a completed meaningful diff or contested design. Review findings are evidence to assess, not instructions to apply automatically. After a fix, rerun the affected checks and re-review the changed contract. Do not repeat unrelated expensive suites when no new change or unresolved concern justifies them.

Apply [unslop](../pstack-unslop/SKILL.md) to authored explanations. Report the resulting behavior, the reason for the change, commands and outcomes, and material native/device gaps. Refer only to evidence actually read or produced. Use the repository's existing completion gate when an exact-HEAD receipt is required. Do not add a parallel verification ledger or workflow.

Adapted from pstack at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`. Original paths and hashes are in [sources.json](sources.json). See [LICENSE](LICENSE).

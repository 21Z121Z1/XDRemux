---
name: pstack-workflow
description: Coordinate a scoped XDRemux investigation, bug fix, feature, or refactor using pstack's reproduce, design, implement, verify, and review sequence. Use when the user requests pstack or when a substantial engineering task needs this workflow. Skip ceremonial planning for a small mechanical edit.
---

# XDRemux engineering workflow

Read [AGENTS.md](../../../AGENTS.md), the responsible implementation, and nearby consumer tests before choosing a change. Use this skill to organize work within the user's current scope and the active session's permissions.

This skill grants no additional authority for external or repository actions. These include messages, ticket updates, publication, PR creation or merge, commits, pushes, and branch deletion. Tool installation, billing changes, and protection settings also require authority from the task or session.

## Define the work

1. State the requested result and current behavior. Identify the producer, consumer, and evidence needed to distinguish success from failure.
2. Read the matching [playbook](references/playbooks.md). Keep an investigation read-only. Preserve behavior in a refactor. Identify any intended behavior change.
3. Before changing a contract between functions, name its data shape, responsible module, and boundary. Compare alternatives when the tradeoff matters. Do not add an abstraction for one mechanical edit.
4. Read [engineering principles](../pstack-engineering-principles/SKILL.md). For a bug with a cheap executable reproduction, read [TDD](../pstack-tdd/SKILL.md).
5. For multi-step work, keep a short plan. Include blocking evidence, independent tasks, shared files or state, and the smallest useful check for each unit. Use one owner when delegation adds coordination without improving coverage.

## Implement and inspect

Make the smallest complete change supported by evidence. Preserve caller contracts, public fixture bytes and provenance, source facts, and safe publication on failure. After another worker changes shared files, read the new state. Inspect the actual diff and its consumers before accepting delegated work.

Use the tools available in the session. Delegate only when the task or applicable instructions authorize delegation and the work has an independent scope. Give each worker the target files, intended behavior, constraints, and required evidence. Serialize writes to shared files.

Inherit the active session's model and settings unless the session explicitly selects different settings. Do not use this skill to select a model, reasoning budget, or service tier.

## Verify and review

Follow the [verification checklist](references/verification.md) and [feature recipes](../xdremux-verify/SKILL.md). Exercise the affected public entry point or inspect the actual artifact. Run the smallest complete set of repository checks.

Compilation, structural parsing, native framework readback, and a real-device edit establish different claims. Report an unavailable host as unavailable. Report a failed check as failed.

For a completed meaningful diff or contested design, use [review](../pstack-review/SKILL.md). Assess findings before applying them. After a fix, rerun affected checks and review the changed contract again. Repeat an expensive suite only when a new change or unresolved concern requires it.

Apply [unslop](../pstack-unslop/SKILL.md) to explanations. Report the resulting behavior, reason for the change, executed commands, results, and material native or device gaps. Cite only evidence read or produced during the task.

When an exact-HEAD receipt is required, use the existing completion gate. Do not add another verification ledger or workflow.

Adapted from pstack at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`. Original paths and hashes are in [sources.json](sources.json). See [LICENSE](LICENSE).

# XDRemux Execution Plans

English | [简体中文](README.md)

Use an execution plan only when work must survive one agent session or one pull request. Do not create a plan for a small change that can be understood from its PR and tests.

An execution plan is recoverable working state. It is not a transcript, chain of thought, diary, or replacement for normative architecture.

## When to create one

Create an active plan when at least one condition applies:

- the work spans multiple capabilities or architectural layers;
- the work is expected to require multiple commits or pull requests;
- required evidence is blocked on a runner, device, private fixture, or external consumer;
- another agent must be able to resume without reconstructing decisions from chat history;
- the work contains a migration or research promotion gate that will be reached incrementally.

For a single bounded PR, use the pull-request task ledger instead.

## Location and lifecycle

Store active plans under `docs/exec-plans/active/` when the first active plan is needed. Move a finished plan to `docs/exec-plans/completed/` only when its completion evidence is recorded.

Do not keep a plan active because future related work might exist. Close the plan when its stated objective is complete and open a new plan for a materially different objective.

## Required fields

Each plan must contain these fields:

- **Status**: `proposed`, `active`, `blocked`, `complete`, or `superseded`.
- **Target capability / layer**: identifiers from `docs/agent-map.json` and the owning architectural layer.
- **Branch / intended base / last verified HEAD**: exact refs, not approximate dates.
- **Objective**: the product or architecture outcome, not a list of files to edit.
- **Invariant**: behavior or boundary that must remain true.
- **Known facts and evidence**: reproducible facts with links to code, fixtures, model cards, tests, or validation records.
- **Decisions**: durable decisions and their evidence. Record the conclusion, not private reasoning traces.
- **Work sequence**: ordered steps with dependencies and acceptance checks.
- **Completed evidence**: exact commands, workflow checks, receipts, or device evidence already obtained.
- **Residual gaps**: what is not proven yet and what evidence would close the gap.
- **Next action**: one concrete resumable action.

## Update discipline

Update the plan when a decision, verified fact, blocker, promotion state, or next action changes. Do not rewrite verified history to match a later conclusion.

Keep volatile facts out when they can be derived cheaply. For branch divergence and current HEAD, use:

```bash
python3 scripts/agent_context.py status
```

For capability ownership and evidence routing, use:

```bash
python3 scripts/agent_context.py capability engine.plan
```

A plan can record the last verified HEAD for reproducibility, but it must not pretend that the value is still current after the branch moves.

## Minimal template

```markdown
# <Outcome>

Status: active
Target capability / layer: engine.plan / Layer 3
Branch: <branch>
Intended base: <base>
Last verified HEAD: <sha>

## Objective

## Invariant

## Known facts and evidence

## Decisions

## Work sequence

1. <step> — acceptance: <check>

## Completed evidence

## Residual gaps

## Next action
```

When a plan discovers a stable repository-wide rule, promote that rule into the architecture, validation contract, model card, or other normative owner. Do not leave stable system knowledge trapped only in a completed plan.

# XDRemux Agent Acceptance Contract

English | [简体中文](AGENTS.zh-CN.md)

An agent must not claim that a change is complete until the required evidence passes for the exact committed `HEAD`.

This file defines the repository acceptance contract. Use the [validation runbook](docs/validation/README.en.md) for the plan format and examples.

## Required sequence

1. Identify each affected product path.
2. Identify the acceptance criteria and required evidence for each path.
3. Make the intended change without unrelated edits.
4. Commit the change.
5. Create a completion-gate plan.
6. Run the gate against the intended base.
7. Verify the generated receipt.
8. Report only the behavior that the evidence proves.

Example:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

A compiler pass, parser pass, or smoke test is not a substitute for the required gate.

## Evidence requirements

Every source change must have a targeted regression check that would fail for the original defect or contract violation.

Every production conversion-core or app-core change must also have functional, integration, or device evidence that reaches the changed behavior.

If more than one entry point changes, validate each affected entry point.

Do not use a static source check as functional evidence.

Do not relabel a static check as a regression or functional check to satisfy the gate.

Strict ISO parser success alone is not acceptance evidence for OPPO Gallery behavior. Keep structural, ImageIO, renderer, and device evidence distinct.

Do not use container structure alone as evidence for interactive Apple Photos editing.

A device-dependent product claim requires device evidence. If the required device or closed component is unavailable, report the device-dependent claim as blocked or explicitly limit the claim to tested offline behavior. Do not mark the device-dependent claim complete without device evidence.

All checks declared in a completion plan are mandatory.

## Scope

Use targeted verification by default.

Run broader repository verification for release or preflight work, cross-module changes, or verification-framework changes.

Do not run unrelated expensive checks only to make a plan look more complete.

## Receipt integrity

The completion receipt is bound to:

- `HEAD`;
- the base commit;
- changed paths;
- a clean tracked worktree;
- declared checks and their results.

A later commit or tracked edit invalidates the receipt.

## Media and fixtures

Public Motion Photo fixtures are versioned under `fixtures/`.

Other large, private, device-only, or Apple-feature samples can remain outside Git.

A verification plan can reference an external local sample when the runner can access it.

Verification receipts under `.codex/verification-receipts/` remain ignored by Git.

## Documentation

Current technical documents follow [docs/style-guide.en.md](docs/style-guide.en.md).

When a code change alters a documented contract, update the English canonical document first and then update the Chinese version.

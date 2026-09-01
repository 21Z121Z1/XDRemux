# Validation Runbook

English | [简体中文](README.md)

Use this directory for validation rationale, acceptance criteria, and reusable evidence records.

Keep executable tests under `Tests/` or `scripts/`.

## Evidence classes

Evidence class answers **what kind of behavior did the check reach?** Keep these classes separate:

| Class | Example | What it can prove |
| --- | --- | --- |
| Static | source or documentation policy check | Text, structure, or architecture contract. |
| Regression | targeted test for a known defect | The specified defect does not reproduce under the tested condition. |
| Functional | real conversion or equivalent media fixture | The affected product path runs on representative data. |
| Integration | framework or app integration | Multiple components work together in the tested environment. |
| Device | real gallery, Photos, display, or device test | Device-dependent behavior in that exact environment. |

A stricter class can include lower-level checks, but it does not change what an unrelated check proves.

## Evidence roles

Evidence role answers **how may this result be used?** This is independent from evidence class.

| Role | Purpose | Acceptance use |
| --- | --- | --- |
| Required gate | Merge/release/completion requirement for a defined scope | Must pass on the exact committed `HEAD`. |
| Promotion evidence | Evidence required to move a capability, model, or adapter to a stronger supported state | Counts only for the promotion rule that names it. |
| Diagnostic probe | Characterization of a dependency, environment, hypothesis, or unknown behavior | Does not count as completion or promotion by itself. |

A diagnostic probe may deliberately use temporary instrumentation, environment-specific commands, or an in-workflow source patch to isolate a problem. That is useful for discovery, but it is not a stable product contract.

Before a diagnostic result becomes required or promotion evidence, encode the finding in the actual implementation, fixture, test, or supported-environment contract and run that reproducible check without hidden diagnostic mutations.

Workflow color alone is not evidence semantics. A green diagnostic workflow is still diagnostic. A red diagnostic workflow can reveal an external limitation without proving the product is broken. Read the role and failing step before drawing a product conclusion.

## Completion gate

Repository agents use `scripts/agent_completion_gate.py` after the intended change is committed.

Example:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

The plan uses an argument array for each command. The gate does not add implicit shell parsing.

Example plan:

```json
{
  "schema_version": 1,
  "scope": "Update current CLI documentation",
  "checks": [
    {
      "name": "documentation-policy",
      "kind": "static",
      "command": [
        "python3",
        "-m",
        "unittest",
        "Tests.test_public_documentation"
      ],
      "timeout_seconds": 120
    }
  ]
}
```

If shell composition is necessary, make the shell explicit in the command array.

Only required checks belong in a completion-gate plan. Keep exploratory probes outside the plan until their result has been promoted to a reproducible acceptance check.

## Receipt contract

A receipt is bound to:

- the current `HEAD`;
- the selected base commit;
- changed paths;
- a clean tracked worktree;
- the exit status and bounded output of each declared check.

A later commit or tracked edit invalidates the receipt.

All checks in the plan are mandatory.

## Select checks by change

Use targeted evidence by default.

A documentation-only change normally needs documentation consistency and link checks.

A conversion-core change normally needs a targeted regression plus functional media evidence.

A Motion Photo change should use the public fixture gates when they cover the affected parser, writer, timing, or publication behavior.

An Apple Photos interaction claim needs native-framework or device evidence that reaches that behavior.

A codec or platform-adapter change should distinguish pure contract tests from real-provider probes. Advertised library support is not enough when runtime capability is the product claim.

Do not run an expensive unrelated matrix only to increase the number of checks.

## CI naming and composition

As the Rust product line matures, make check purpose visible from the workflow/job or its documentation:

- required product/merge gates should have stable names;
- capability promotion checks should identify the capability they promote;
- diagnostic probes should be recognizable as diagnostic and should not silently become required checks;
- the release/product gate should compose capability evidence instead of re-implementing it.

The objective is not to minimize workflow count. It is to make the evidence graph easy for an agent to interpret.

## Public and private media

The repository contains versioned real Motion Photo fixtures under `fixtures/`.

Other ProXDR, Apple-feature, or device-only samples can remain outside Git.

A verification plan can reference an external local sample by absolute path when the runner has that file.

## Historical validation records

Files with a date in this directory can describe an older implementation state.

Do not edit old measurements to match the current implementation. Write a new current document or a new dated record when the evidence changes.

Current documentation follows the [technical writing guide](../style-guide.en.md).

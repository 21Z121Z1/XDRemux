# Validation Runbook

English | [简体中文](README.md)

Use this directory for validation rationale, acceptance criteria, and evidence records.

Keep executable tests under `Tests/` or `scripts/`.

## Evidence classes

Keep these classes separate:

| Class | Example | What it can prove |
| --- | --- | --- |
| Static | source or documentation policy check | Text, structure, or architecture contract. |
| Regression | targeted test for a known defect | The specified defect does not reproduce under the tested condition. |
| Functional | real conversion or equivalent media fixture | The affected product path runs on representative data. |
| Integration | framework or app integration | Multiple components work together in the tested environment. |
| Device | real gallery, Photos, display, or device test | Device-dependent behavior in that exact environment. |

A stricter class can include lower-level checks, but it does not change what an unrelated check proves.

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

## Receipt contract

A receipt is bound to:

- the current `HEAD`;
- the selected base commit;
- the changed-file set;
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

Do not run an expensive unrelated matrix only to increase the number of checks.

## Public and private media

The repository contains versioned real Motion Photo fixtures under `fixtures/`.

Other ProXDR, Apple-feature, or device-only samples can remain outside Git.

A verification plan can reference an external local sample by absolute path when the runner has that file.

## Historical validation records

Files with a date in this directory can describe an older implementation state.

Do not edit old measurements to match the current implementation. Write a new current document or a new dated record when the evidence changes.

Current documentation follows the [technical writing guide](../style-guide.en.md).

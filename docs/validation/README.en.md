# Run validation and verify a completion receipt

English | [简体中文](README.md)

This directory holds validation rationale, acceptance criteria, and evidence records. Executable tests belong in `tests/` or `scripts/`. Use this procedure to record checks for a committed change.

## Select the evidence class

| Class | Example | Scope of the result |
| --- | --- | --- |
| `static` | Source or documentation policy check. | Text, structure, or architecture contracts. |
| `regression` | A targeted test for a known defect. | The defect does not reproduce under the tested conditions. |
| `functional` | Real conversion or an equivalent media fixture. | The affected product path runs on representative data. |
| `integration` | Framework or app integration. | Components work together in the tested environment. |
| `device` | A gallery, Photos, display, or real-device test. | Device-dependent behavior in that environment. |

A higher-level test can include lower-level checks. It does not extend the scope of unrelated evidence. For example, a framework readback does not prove a Photos edit/save/reopen interaction.

## Prepare and run the completion gate

Use [`scripts/ci/agent_completion_gate.py`](../../scripts/ci/agent_completion_gate.py) after committing the intended change. The tracked worktree must be clean. Every check declared in the plan is mandatory.

1. Select checks for the behavior that changed.
2. Write a plan with an argument array for each command.
3. Run the gate against the intended base.
4. Verify the resulting receipt.

A documentation plan can use the existing policy and link checks:

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
        "tests.test_public_documentation",
        "tests.validation.test_repository_layout"
      ],
      "timeout_seconds": 120
    }
  ]
}
```

Save the plan as `/tmp/xdremux-agent-verification.json` for this example. Run these commands from the repository root:

```bash
python3 scripts/ci/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/ci/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

The gate does not add shell interpretation to the argument arrays. If a command needs shell composition, invoke the shell explicitly in its array.

## Understand what the receipt binds

A completion receipt identifies:

- The current `HEAD` and the selected base commit.
- The changed-file set.
- A clean tracked worktree.
- Each declared check's exit status and bounded output.

A later commit or tracked edit invalidates the receipt. Re-run the appropriate checks after such a change.

The gate treats canonical Rust `crates/` and Apple adapter or app sources as product implementation. Source changes require regression evidence. Product changes also require functional, integration, or device evidence.

Cargo and Swift manifests, lockfiles, toolchain configuration, and the app's Xcode project are build inputs. They are not documentation-only changes. Deletions count, as do both sides of a move across component ownership. Git paths and tracked status use NUL-delimited records. Quoting, embedded whitespace, or arbitrary path bytes cannot hide a change.

A research `.rs` file remains research code. Rust syntax alone does not make that file product implementation.

## Use targeted checks

| Change | Evidence to select |
| --- | --- |
| Documentation only | Documentation consistency and local-link checks. |
| Conversion core | A targeted regression and functional media evidence. |
| Motion Photo | Public fixture checks for the affected parser, writer, timing, or publication behavior. |
| Photos interaction | Native framework or device evidence that reaches the stated interaction. |

Do not run an unrelated expensive matrix only to increase the check count. The [testing policy](../quality/testing.en.md) gives the full selection rules.

## Handle public and private media

Versioned real Motion Photo files are in `fixtures/`. Other ProXDR, Apple-feature, and device-only samples can remain outside Git.

A plan can reference an external sample by absolute path if the runner can read that file. Keep the sample's privacy and the artifact-upload limits in the testing policy.

## Preserve historical validation

Dated files in this directory can describe an older implementation. Do not change old measurements to match current code. When evidence changes, write a new current document or a new dated record.

Current documentation follows the [technical writing guide](../style-guide.en.md).

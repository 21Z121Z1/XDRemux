# Validation

This directory is for validation notes that explain how XDRemux outputs should be checked.

Good validation documents should distinguish:

- Structural checks: HEIF/ISOBMFF boxes, item references, metadata placement, gain-map association.
- Renderer checks: ImageIO recognition, Apple Photos behavior, Android/OPPO Gallery behavior.
- Regression checks: output hashes, metadata snapshots, and known sample behavior.
- Device checks: real-device observations such as HDR badge visibility or EDR brightness changes.

Keep actual test executables under `Tests/` or `scripts/`; keep the rationale, acceptance criteria, and runbooks here.

Current encoding audit:

- [Active encoding quality and size Pareto audit (2026-07-18)](encoding-quality-pareto-20260718.md)

## Agent completion gate

Agents must use `scripts/agent_completion_gate.py` before declaring a committed
change complete. The gate is deliberately separate from any one build system:
it executes the checks selected for the actual change and writes a JSON receipt
bound to the current commit.

Run it only after committing the intended change and returning the tracked
worktree to a clean state:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

The verification plan uses this schema:

```json
{
  "schema_version": 1,
  "scope": "Fix default Swift CLI conversion for an existing ISO gain-map input",
  "checks": [
    {
      "name": "swift-cli-build",
      "kind": "static",
      "command": ["swift", "build", "--product", "xdremux"],
      "timeout_seconds": 900
    },
    {
      "name": "existing-iso-default-regression",
      "kind": "regression",
      "command": [
        "python3",
        "Tests/validation/verify_swift_cli_sample.py",
        "--input",
        "/absolute/path/to/existing-iso-sample.heic",
        "--expected-pixel-format",
        "444f"
      ],
      "timeout_seconds": 300
    },
    {
      "name": "real-sample-output-matrix",
      "kind": "functional",
      "command": [
        "python3",
        "Tests/validation/verify_swift_cli_sample.py",
        "--input",
        "/absolute/path/to/private-uhdr-sample.heic",
        "--expected-pixel-format",
        "420f",
        "--oppo-compatible"
      ],
      "timeout_seconds": 900
    }
  ]
}
```

`command` is an argument array and is executed without implicit shell parsing.
If shell composition is genuinely required, make it explicit with
`["/bin/zsh", "-lc", "..."]`. An optional `env` object may provide per-check
string environment variables.

`Tests/validation/verify_swift_cli_sample.py` is the reusable real-sample
harness for Swift CLI plans. It builds the production SwiftPM `xdremux` product,
converts a temporary output (or temporary in-place copy), and asks ImageIO to
assert the expected gain-map pixel format. Private samples remain outside Git.

Kinds are `static`, `regression`, `functional`, `integration`, and `device`.
All listed checks are mandatory. Source changes require a `regression` check;
changes under `xdremux/` or the macOS app `Sources/` additionally require a
`functional`, `integration`, or `device` check. The gate also requires:

- a non-empty diff against the selected base;
- `git diff --check` success;
- a clean tracked worktree before and after checks;
- an unchanged `HEAD` while checks run;
- zero exit status from every declared check.

Receipts record commands, durations, exit status, bounded output tails, the
base/head commits, and changed paths. A later commit or tracked edit makes the
receipt unverifiable. Missing device access is not a pass: either restrict the
claim to offline behavior or report the device-dependent acceptance as blocked.

# Validation

This directory is for validation notes that explain how XDRemux outputs should be checked.

Good validation documents should distinguish:

- Structural checks: HEIF/ISOBMFF boxes, item references, metadata placement, gain-map association.
- Renderer checks: ImageIO recognition, Apple Photos behavior, Android/OPPO Gallery behavior.
- Regression checks: output hashes, metadata snapshots, and known sample behavior.
- Device checks: real-device observations such as HDR badge visibility or EDR brightness changes.

Keep actual test executables under `Tests/` or `scripts/`; keep the rationale, acceptance criteria, and runbooks here.

## Agent completion gate

Agents must use `scripts/agent_completion_gate.py` before declaring a committed
change complete. The gate is deliberately separate from any one build system:
it executes the checks selected for the actual change and writes a JSON receipt
bound to the current commit.

Run it only after committing the intended change and returning the tracked
worktree to a clean state:

```bash
python3 scripts/agent_completion_gate.py run \
  --base <verified-base> \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

The verification plan uses this schema:

```json
{
  "schema_version": 1,
  "scope": "Fix default Swift CLI conversion for an existing ISO gain-map input",
  "change_impact": "output",
  "impact_rationale": "Changes the default conversion request and generated HEIF output",
  "checks": [
    {
      "name": "swift-cli-typecheck",
      "kind": "static",
      "command": ["swiftc", "-typecheck", "xdremux/swift-cli/XDRemux.swift"],
      "timeout_seconds": 120
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

New plans should declare one of these impact levels:

| `change_impact` | Use when | Minimum evidence |
| --- | --- | --- |
| `documentation` | Only README, docs, documentation assets, or the public-doc harness changes | Link/structure/static checks; public projection when applicable |
| `non_output` | Runtime or tooling changes are proven not to change conversion requests or generated files | Targeted regression and only the build/integration checks for affected entry points |
| `output` | Algorithms, defaults, writer behavior, metadata, request mapping, or validation can alter generated files | Regression plus real functional, integration, or device evidence |
| `release` | Release/preflight or broad product acceptance | Regression, functional, and integration matrix; device evidence for device claims |

`impact_rationale` is required for every explicit impact. It should say why
output files can or cannot change. `auto` is accepted only for compatibility
with older plans and applies the conservative legacy policy.

Examples:

- README wording: `documentation`; run documentation tests, link checks, and
  projection dry-run. Do not run photo conversion or App signing.
- CLI progress rendering: `non_output`; run renderer, TTY, localization, and
  CLI smoke tests. Do not run fixture conversion when requests and files cannot
  change.
- SwiftUI layout: `non_output`; build the App and run affected model/view
  tests. Do not run the HDR matrix.
- HEIF box order or Gain Map encoding: `output`; run focused unit tests and a
  real output comparison.
- Release candidate: `release`; run the complete required matrix.

`command` is an argument array and is executed without implicit shell parsing.
If shell composition is genuinely required, make it explicit with
`["/bin/zsh", "-lc", "..."]`. An optional `env` object may provide per-check
string environment variables.

`Tests/validation/verify_swift_cli_sample.py` is the reusable real-sample
harness for Swift CLI plans. It compiles the production CLI, converts a
temporary output (or temporary in-place copy), and asks ImageIO to assert the
expected gain-map pixel format. Private samples remain outside Git.

Kinds are `static`, `regression`, `functional`, `integration`, and `device`.
All listed checks are mandatory. Source changes require a `regression` check.
Explicit `output` changes additionally require functional, integration, or
device evidence. Explicit `non_output` changes do not require real-photo
evidence. The gate also requires:

- a non-empty diff against the selected base;
- `git diff --check` success;
- a clean tracked worktree before and after checks;
- an unchanged `HEAD` while checks run;
- zero exit status from every declared check.

## Selecting the base

The receipt covers the diff from `--base` to the exact committed `HEAD`.

- Use the target branch merge base, commonly `origin/main`, for the first
  commit in a change series.
- A follow-up commit may use its direct parent when that parent already passed
  and had its receipt independently verified. This prevents a narrow docs or
  cleanup commit from re-running already accepted product evidence.
- Do not select an arbitrary recent commit merely to hide affected files. The
  base must represent previously accepted state.

The receipt records the resolved base commit and changed-file set, so reviewers
can audit incremental verification.

Receipts record impact classification, commands, durations, exit status,
bounded output tails, the
base/head commits, and changed paths. A later commit or tracked edit makes the
receipt unverifiable. Missing device access is not a pass: either restrict the
claim to offline behavior or report the device-dependent acceptance as blocked.

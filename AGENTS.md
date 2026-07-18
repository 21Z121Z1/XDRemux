# XDRemux Agent Acceptance Contract

## Completion claims are gated

An agent must not say a change is complete, accepted, ready, or fully working
until the repository completion gate passes for the exact committed `HEAD`.
Passing an individual compiler, parser, or smoke command is not a substitute
for the gate.

The required flow is:

1. Identify the affected product paths and their acceptance criteria.
2. Implement and commit the intended changes without unrelated files.
3. Create a verification plan using the schema documented in
   `docs/validation/README.md`.
4. Select the base for this completed change:
   - use the target branch merge base for the first change in a series;
   - use an already verified parent commit for a follow-up commit that changes
     only a narrower scope.
5. Run:

   ```bash
   python3 scripts/agent_completion_gate.py run \
     --base <verified-base> \
     --plan /tmp/xdremux-agent-verification.json
   ```

6. Confirm the generated receipt independently:

   ```bash
   python3 scripts/agent_completion_gate.py verify \
     .codex/verification-receipts/$(git rev-parse HEAD).json
   ```

The receipt is bound to `HEAD`, the base commit, the changed-file set, and a
clean tracked worktree. Any later commit or tracked edit invalidates it.

Do not automatically use a stale `origin/main` when the direct parent already
has a verified receipt. That expands the changed-file set and can incorrectly
repeat evidence for work that was already accepted.

## Change impact

Every new verification plan should declare `change_impact` and a concrete
`impact_rationale`:

| Impact | Examples | Required evidence |
| --- | --- | --- |
| `documentation` | README, docs, documentation links and examples | Documentation regression/static checks and public projection when applicable |
| `non_output` | UI copy, terminal rendering, logs, CI, build tooling, code organization proven not to alter conversion requests or files | Targeted regression plus build/integration checks only for affected entry points |
| `output` | HDR algorithms, Gain Map encoding, HEIF writer, metadata, defaults, request mapping, output validation | Targeted regression plus real functional/integration evidence |
| `release` | Release candidate, broad cross-module delivery, device compatibility claim | Regression, functional, and integration matrix; device evidence for device claims |

`auto` remains available for old plans and applies the conservative legacy
policy. Use an explicit impact for new work.

## Required evidence

- Every source change needs a targeted regression check that would fail for
  the original defect or contract violation.
- Real-photo or device evidence is required when output files, conversion
  requests, defaults, metadata, container layout, or device-facing behavior
  can change. It is not required for a documented `non_output` change.
- UI-only App changes need the relevant App build or model test, not an HDR
  output matrix. CLI text/localization changes need renderer/parser tests, not
  photo conversion. Build and CI changes need their script/workflow tests.
- The verification plan must cover every affected entry point. If Swift CLI,
  Python CLI, and macOS app behavior all change, all three need evidence.
- A device-dependent product claim requires device evidence. If the device or
  closed component is unavailable, report the task as blocked or explicitly
  limit the claim to offline behavior; do not mark the device claim complete.
- Strict ISO parser success alone is not acceptance evidence for OPPO Gallery
  behavior. Keep structural, ImageIO, renderer, and device gates distinct.
- All declared checks are mandatory. Do not relabel a static check as a
  regression or functional check to satisfy the gate.

## Scope

Use targeted verification by default. Full repository verification is needed
for release/preflight work, broad cross-module output changes, or an explicit
full-product claim. Verification-framework changes require focused framework
tests and workflow validation; they do not require the photo matrix unless
they also change or claim photo output behavior. The gate enforces evidence
completeness; it does not justify running unrelated expensive checks.

Large/private media stays outside Git. Verification plans may reference local
fixtures, but receipts under `.codex/verification-receipts/` remain ignored.

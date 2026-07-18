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
4. Run:

   ```bash
   python3 scripts/agent_completion_gate.py run \
     --base origin/main \
     --plan /tmp/xdremux-agent-verification.json
   ```

5. Confirm the generated receipt independently:

   ```bash
   python3 scripts/agent_completion_gate.py verify \
     .codex/verification-receipts/$(git rev-parse HEAD).json
   ```

The receipt is bound to `HEAD`, the base commit, the changed-file set, and a
clean tracked worktree. Any later commit or tracked edit invalidates it.

## Required evidence

- Every source change needs a targeted regression check that would fail for
  the original defect or contract violation.
- Every production converter or app-core change also needs a real functional,
  integration, or device check. Type-checking and syntax checks are static
  checks, not functional checks.
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
for release/preflight work, cross-module changes, or verification-framework
changes. The gate enforces evidence completeness; it does not justify running
unrelated expensive checks.

Large/private media stays outside Git. Verification plans may reference local
fixtures, but receipts under `.codex/verification-receipts/` remain ignored.

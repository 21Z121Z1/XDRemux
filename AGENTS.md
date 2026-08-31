# XDRemux Agent Operating and Acceptance Contract

English | [简体中文](AGENTS.zh-CN.md)

An agent must understand the affected system boundary before it changes code, and it must not claim that a change is complete until the required evidence passes for the exact committed `HEAD`.

Use the [system architecture](docs/architecture.en.md) to identify ownership and dependencies. Use the [transition roadmap](docs/roadmap.en.md) for Rust migration and Photographic Styles research promotion. Use the [validation runbook](docs/validation/README.en.md) for completion-plan format and evidence examples.

## Bootstrap sequence

For a substantial task, establish the repository state before broad search or implementation:

1. Record the current branch, exact `HEAD`, intended base, and clean/dirty state.
2. Read `docs/architecture.en.md`.
3. Read `docs/roadmap.en.md` when the task touches the Rust rewrite, a long-lived branch, or model research.
4. Identify the affected capability identifiers and architectural layers.
5. Compare the active branch with its intended base before assuming that either side contains all current knowledge.
6. Read the owning modules, neighboring tests, relevant fixtures, and current normative documents.
7. Define acceptance criteria before implementation when a task crosses layers or changes a contract.

Do not start by scanning the whole repository unless the task is itself repository-wide. Expand the working set only when evidence shows that another layer is involved.

## Source-of-truth discipline

Use the source that is authoritative for the question:

- released user behavior: current release contract, current public documentation, implementation, and matching evidence;
- architectural ownership: `docs/architecture.en.md` and current code boundaries;
- active branch facts: that branch plus an explicit comparison with its intended base;
- behavioral invariants: standards, fixtures, conformance tests, and device evidence where applicable;
- research claims: model cards, data provenance, evaluation code, and current measured artifacts;
- dated audits and old PR descriptions: historical evidence unless a current document adopts the conclusion.

An old implementation is a useful migration oracle, not automatically the specification.

No long-lived branch may be the only place where a stable system contract is recorded.

## Required implementation sequence

1. Identify each affected capability and product path.
2. Identify the acceptance criteria and required evidence for each path.
3. Make the intended change without unrelated edits.
4. Add or update the targeted regression or conformance evidence.
5. Update current normative documentation when a contract changes.
6. Commit the change.
7. Create a completion-gate plan.
8. Run the gate against the intended base.
9. Verify the generated receipt.
10. Report only the behavior that the evidence proves.

Example:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

A compiler pass, parser pass, or smoke test is not a substitute for the required gate.

## Architecture rules

Keep the dependency rules in `docs/architecture.en.md` during implementation.

In particular:

- do not organize the architecture by programming language;
- do not move product policy into parsers, codec adapters, native helpers, CLI code, or model predictors;
- keep planning deterministic and free of platform I/O;
- prefer operation-scoped adapter capabilities over a monolithic backend;
- do not create a Rust crate only to mirror an old Swift directory;
- treat publication, provenance, collision handling, and crash recovery as correctness contracts;
- treat research model output as a candidate until its promotion gates pass;
- do not add a permanent cross-language runtime dependency only to make migration easier.

When a change does not fit the current architecture, first decide which layer should own the behavior. Do not solve ownership ambiguity with another special case.

## Evidence requirements

Every source change must have a targeted regression or conformance check that would fail for the original defect or contract violation.

Every production conversion-core or app-core change must also have functional, integration, or device evidence that reaches the changed behavior.

If more than one entry point changes, validate each affected entry point.

Do not use a static source check as functional evidence.

Do not relabel a static check as a regression or functional check to satisfy the gate.

Strict ISO parser success alone is not acceptance evidence for OPPO Gallery behavior. Keep structural, ImageIO, renderer, and device evidence distinct.

Do not use container structure alone as evidence for interactive Apple Photos editing.

A device-dependent product claim requires device evidence. If the required device or closed component is unavailable, report the device-dependent claim as blocked or explicitly limit the claim to tested offline behavior. Do not mark the device-dependent claim complete without device evidence.

All checks declared in a completion plan are mandatory.

## Migration evidence

For a capability that moves from Swift/Python to Rust, record:

1. the normalized contract;
2. the old implementation or external evidence used as an oracle;
3. the Rust owner;
4. the promotion evidence.

Cross-implementation parity is not enough when the old implementation conflicts with an external standard or stronger evidence.

The v1.4 Swift/Python line is a bounded released reference. Do not keep adding large new architecture to it only to preserve implementation symmetry with Rust.

## Research promotion

A model, learned heuristic, or research-only producer must remain outside production defaults until the applicable promotion gates in `docs/roadmap.en.md` pass.

Keep training provenance, leakage controls, held-out/OOD results, consumer correlation, uncertainty/fallback behavior, end-to-end evidence, and operational budget separate.

Do not interpret lower offline loss as sufficient product evidence.

## Scope

Use targeted verification by default.

Run broader repository verification for release or preflight work, cross-module changes, architecture/control-plane changes, or verification-framework changes.

Do not run unrelated expensive checks only to make a plan look more complete.

## Receipt integrity

The completion receipt is bound to:

- `HEAD`;
- the base commit;
- changed paths;
- a clean tracked worktree;
- declared checks and their results.

A later commit or tracked edit invalidates the receipt.

## Branch lifecycle

Before deleting or retiring a long-lived branch:

1. compare it with its intended destination;
2. identify implementation, contracts, evidence, or provenance that exist only on that branch;
3. promote reusable current knowledge into code, tests, model cards, or normative documentation;
4. preserve useful dated experiments as historical records;
5. retire the branch only when no stable contract depends on branch-only knowledge.

A merged implementation is not sufficient if the maintenance reasoning or evidence boundary exists only in an old PR, branch, or chat transcript.

## Media and fixtures

Public Motion Photo fixtures are versioned under `fixtures/`.

Other large, private, device-only, or Apple-feature samples can remain outside Git.

A verification plan can reference an external local sample when the runner can access it.

Verification receipts under `.codex/verification-receipts/` remain ignored by Git.

## Documentation

Current technical documents follow [docs/style-guide.en.md](docs/style-guide.en.md).

When a code change alters a documented contract, update the English canonical document first and then update the Chinese version.

Do not persist transient chain-of-thought or session scratch in the repository. Persist decisions, contracts, provenance, reusable evidence, and plans that another maintainer or agent must recover later.

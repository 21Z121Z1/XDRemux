# XDRemux Agent Operating Contract

English | [简体中文](AGENTS.zh-CN.md)

This file is the low-cost entry point for repository agents. Keep the working set small, then expand it only when the affected contract requires more context.

Use these canonical documents instead of reconstructing the system from directory names or old PRs:

- [System architecture](docs/architecture.en.md): layers, capability ownership, branch roles, and source-of-truth rules.
- [Transition roadmap](docs/roadmap.en.md): Rust migration state and Photographic Styles research promotion.
- [Validation runbook](docs/validation/README.en.md): evidence classes, completion plans, and receipts.

## Bootstrap

For a substantial task:

1. Record the current branch, exact `HEAD`, intended base, and clean/dirty state.
2. Identify the affected capability and layer from `docs/architecture.en.md`.
3. Compare the active branch with its intended base before assuming either side is current.
4. Read the owning module, neighboring tests, relevant fixtures, and current normative document.
5. Read `docs/roadmap.en.md` only when the task touches migration, a long-lived branch, or model research.
6. Define acceptance criteria before implementation when the task crosses layers or changes a contract.

Do not scan the whole repository by default. Expand the working set only when evidence shows that another layer is involved.

## Source of truth

Use the narrowest authoritative source:

- released behavior: current release contract, current public documentation, implementation, and matching evidence;
- architecture: `docs/architecture.en.md` and current code boundaries;
- active branch facts: that branch plus an explicit comparison with its intended base;
- behavioral invariants: standards, fixtures, conformance tests, and device evidence where applicable;
- research claims: model cards, data provenance, evaluation code, and current measured artifacts.

Treat dated audits, old PR descriptions, and old implementations as evidence, not automatic specifications.

No long-lived branch may be the only place where a stable system contract is recorded.

## Architecture invariants

- Programming languages are implementation lanes, not architecture layers.
- Format primitives do not own media or product policy.
- Normalized media semantics do not depend on CLI, app, or research code.
- Planning is deterministic and free of platform I/O.
- Prefer operation-scoped adapter capabilities over a monolithic backend.
- Publication, provenance, collision handling, and crash recovery are correctness contracts.
- Product shells do not reimplement parsing or media semantics.
- Research outputs are candidates until their promotion gates pass.
- Do not create a Rust crate only to mirror an old Swift directory.

If a change does not fit these boundaries, resolve ownership first instead of adding another special case.

## Completion contract

An agent must not claim that a change is complete until the required evidence passes for the exact committed `HEAD`.

Required sequence:

1. Identify every affected capability and product path.
2. Select the regression, conformance, functional, integration, or device evidence that the claim requires.
3. Make the intended change without unrelated edits and update contract documentation when needed.
4. Commit the change.
5. Run `scripts/agent_completion_gate.py` against the intended base with a plan that contains all required checks.
6. Verify the generated receipt for the exact `HEAD`.
7. Report only behavior that the evidence proves, and state residual gaps explicitly.

Example:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

A compiler, parser, smoke, or static source check is not a substitute for functional evidence when the changed claim is functional.

Keep structural, ImageIO, renderer, integration, and device evidence distinct. Apple Photos or device behavior requires evidence that reaches that consumer. If the required environment is unavailable, limit the claim instead of marking it complete.

All checks declared in the completion plan are mandatory. A later commit or tracked edit invalidates the receipt.

## Migration and research

For each capability moved from Swift/Python to Rust, record four facts in the roadmap, PR, or current contract: normalized contract, oracle/evidence, Rust owner, and promotion evidence.

Cross-implementation parity is useful but is not sufficient when the old implementation conflicts with an external standard or stronger evidence.

The v1.4 Swift/Python line is a bounded released reference. Do not add large new architecture to it only to preserve symmetry with Rust.

A model, learned heuristic, or research-only producer stays outside production defaults until the applicable gates in `docs/roadmap.en.md` pass.

Before retiring a long-lived branch, promote branch-only stable knowledge into code, tests, model cards, or normative documentation. Preserve useful dated experiments as historical records.

## Media and documentation

Public Motion Photo fixtures are versioned under `fixtures/`. Large private, device-only, or Apple-feature samples can remain outside Git when their provenance and use are documented by the relevant validation path.

Current technical documents follow [docs/style-guide.en.md](docs/style-guide.en.md). Update the English canonical document first, then its Chinese translation.

Do not persist transient chain-of-thought or session scratch in the repository. Persist only decisions, contracts, provenance, reusable evidence, and plans that another maintainer or agent must recover later.

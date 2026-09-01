# XDRemux Agent Operating Contract

English | [简体中文](AGENTS.zh-CN.md)

This file is the low-cost entry point for repository agents. Keep the working set small and expand it only when the affected contract requires more context.

Use these sources instead of reconstructing the system from directory names, workflow names, or old PRs:

- [System architecture](docs/architecture.en.md): stable layers, ownership, dependency rules, and source-of-truth rules.
- [`docs/agent-map.json`](docs/agent-map.json): machine-readable capability routing and long-lived branch roles.
- [Transition roadmap](docs/roadmap.en.md): migration and research promotion gates.
- [Validation runbook](docs/validation/README.en.md): evidence classes, evidence roles, completion plans, and receipts.
- [Execution plans](docs/exec-plans/README.en.md): durable resumable state for work that spans sessions or PRs.

## Bootstrap

For a substantial task:

1. Derive current Git state instead of copying it from prose:

   ```bash
   python3 scripts/agent_context.py status
   ```

   If the branch is not registered in `docs/agent-map.json`, pass the intended base explicitly with `--base`.
2. Identify the affected capability and layer. Use `python3 scripts/agent_context.py capability <id>` when the identifier is known.
3. Read the owning module, neighboring tests, relevant fixtures, and the narrowest current normative document.
4. Compare the active branch with its intended base before assuming either side contains all current knowledge.
5. Read `docs/roadmap.en.md` only when the task touches migration, a long-lived branch, or model research.
6. Create an execution plan only when the work must survive one session/PR, spans layers, or has blocked promotion evidence.
7. Define acceptance criteria before implementation when the task crosses layers or changes a contract.

Do not scan the whole repository by default. Expand the working set only when evidence shows another layer is involved.

## Source of truth

Use the narrowest authoritative source:

- released behavior: current release contract, public documentation, implementation, and matching evidence;
- stable architecture: `docs/architecture.en.md`;
- routing metadata and branch roles: `docs/agent-map.json`;
- live branch facts: Git and the branch itself, not copied HEADs, ahead/behind counts, crate lists, or workflow results in prose;
- behavioral invariants: standards, fixtures, conformance tests, and device evidence where applicable;
- research claims: model cards, data provenance, evaluation code, and current measured artifacts.

Treat dated audits, old PR descriptions, completed plans, and old implementations as evidence, not automatic specifications.

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
- Do not duplicate repository-wide rules into path-specific instructions. Add a local instruction only for a real local invariant that cannot be expressed by the owning contract or test.

If a change does not fit these boundaries, resolve ownership first instead of adding another special case.

## Completion contract

An agent must not claim that a change is complete until the required evidence passes for the exact committed `HEAD`.

Required sequence:

1. Identify every affected capability and product path.
2. Select the regression, conformance, functional, integration, or device evidence required by the claim.
3. Separate required or promotion evidence from diagnostic probes. A diagnostic probe does not become acceptance evidence merely because it is useful or green.
4. Make the intended change without unrelated edits and update contract documentation when needed.
5. Commit the change.
6. Run `scripts/agent_completion_gate.py` against the intended base with every required check in the plan.
7. Verify the generated receipt for the exact `HEAD`.
8. Report only behavior that the evidence proves, and state residual gaps explicitly.

Example:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

A compiler, parser, smoke, static check, or diagnostic workflow is not a substitute for functional evidence when the changed claim is functional.

Keep structural, ImageIO, renderer, integration, and device evidence distinct. Apple Photos or device behavior requires evidence that reaches that consumer. If the required environment is unavailable, limit the claim instead of marking it complete.

All checks declared in a completion plan are mandatory. A later commit or tracked edit invalidates the receipt.

## Migration and research

For each capability moved from Swift/Python to Rust, record four facts in the roadmap, PR, current contract, or active execution plan: normalized contract, oracle/evidence, Rust owner, and promotion evidence.

Cross-implementation parity is useful but is not sufficient when the old implementation conflicts with an external standard or stronger evidence.

The v1.4 Swift/Python line is a bounded released reference. Do not add large new architecture to it only to preserve symmetry with Rust.

A model, learned heuristic, or research-only producer stays outside production defaults until the applicable gates in `docs/roadmap.en.md` pass. Stable research protocols belong with research/model documentation, not in the general development guide.

Before retiring a long-lived branch, promote branch-only stable knowledge into code, tests, model cards, or normative documentation. Preserve useful dated experiments as historical records.

## Media and documentation

Public Motion Photo fixtures are versioned under `fixtures/`. Large private, device-only, or Apple-feature samples can remain outside Git when their provenance and use are documented by the relevant validation path.

Current technical documents follow [docs/style-guide.en.md](docs/style-guide.en.md). Update the English canonical document first, then its Chinese translation.

Do not persist transient chain-of-thought or session scratch in the repository. Persist only decisions, contracts, provenance, reusable evidence, and resumable plans that another maintainer or agent must recover later.

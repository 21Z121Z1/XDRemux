# XDRemux Documentation

English | [简体中文](README.md)

Use the [project README](../README.en.md) for installation and common commands.

All current technical documents use the writing policy in the [technical writing guide](style-guide.en.md). English is the canonical source for bilingual current documentation.

## System orientation

Read only what the task needs:

- [System architecture](architecture.en.md): stable abstraction layers, ownership, dependency rules, and source-of-truth rules.
- [`agent-map.json`](agent-map.json): machine-readable capability routing and long-lived branch roles.
- [Transition roadmap](roadmap.en.md): migration/research stages, promotion gates, and branch retirement rules. It intentionally does not store volatile Git state.
- [Agent operating contract](../AGENTS.md): low-cost bootstrap and exact-HEAD completion discipline.
- [Execution-plan contract](exec-plans/README.en.md): resumable state for work that spans sessions/PRs; do not use it for small bounded changes.

For a substantial task, derive live branch state first:

```bash
python3 scripts/agent_context.py status
```

Then route a known capability without scanning the repository:

```bash
python3 scripts/agent_context.py capability engine.plan
```

The programming language is not the architecture boundary; media semantics, capabilities, product contracts, and evidence are.

## User documentation

- [CLI reference](cli.en.md): commands, options, output paths, and exit behavior.
- [Apple features](apple-features.en.md): Photographic Styles, Apple Portrait, and supported combinations.
- [Supported devices](supported-devices.en.md): ProXDR capture compatibility and its limits.

## Developer documentation

- [Development and builds](development.en.md): v1.4 package products, repository layout, app builds, and integration.
- [Testing policy](quality/testing.en.md): required evidence for a change.
- [Regression and real-sample verification](quality/evals.en.md): reusable test and fixture gates.
- [Output policy](quality/logging.en.md): stdout, stderr, JSON output, and error-text rules.
- [Validation runbook](validation/README.en.md): evidence classes, evidence roles, completion-gate plans, and receipts.
- [Test suite guide](../Tests/README.en.md): Swift and Python test entry points.
- [Fixture guide](../fixtures/README.en.md): versioned Motion Photo fixtures and identity rules.

## Technical implementation

- [Technical implementation index](xdremux/README.en.md): stable v1.4 implementation contracts and product-path details.
- [ReverseKey1Ensemble model card](../Models/ReverseKey1Ensemble.model-card.en.md): optional research model contract on the released line.

Active research branches can contain additional model cards and research protocols that have not been promoted to the released line. Treat those branch-local documents as authoritative only for research facts on that branch; promotion into product behavior follows the roadmap gates.

## Historical records

The following files are evidence records. They describe a specific repository state or experiment. They are not current product specifications.

- ISO conformance audit, 2026-05-11: [current-language summary](xdremux/iso-conformance-audit-20260511.summary.en.md) | [original record](xdremux/iso-conformance-audit-20260511.md)
- Encoding quality and size audit, 2026-07-18: [current-language summary](validation/encoding-quality-pareto-20260718.summary.en.md) | [original record](validation/encoding-quality-pareto-20260718.md)
- Vendor Live Photo geometry evidence: [current-language summary](validation/vendor-live-photo-geometry.summary.en.md) | [original record](validation/vendor-live-photo-geometry.md)

Use the current documents above for product behavior. Do not infer a current guarantee from an old path, measurement, implementation note, PR description, completed plan, or branch name in a historical record.

# XDRemux Documentation

English | [简体中文](README.md)

Use the [project README](../README.en.md) for installation and common commands.

All current technical documents use the writing policy in the [technical writing guide](style-guide.en.md). English is the canonical source for bilingual current documentation.

## User documentation

- [CLI reference](cli.en.md): commands, options, output paths, and exit behavior.
- [Apple features](apple-features.en.md): Photographic Styles, Apple Portrait, and supported combinations.
- [Supported devices](supported-devices.en.md): ProXDR capture compatibility and its limits.

## Developer documentation

- [Development and builds](development.en.md): package products, repository layout, app builds, and integration.
- [Testing policy](quality/testing.en.md): required evidence for a change.
- [Regression and real-sample verification](quality/evals.en.md): reusable test and fixture gates.
- [Output policy](quality/logging.en.md): stdout, stderr, JSON output, and error-text rules.
- [Validation runbook](validation/README.en.md): completion-gate plans and evidence classes.
- [Test suite guide](../Tests/README.en.md): Swift and Python test entry points.
- [Fixture guide](../fixtures/README.en.md): versioned Motion Photo fixtures and identity rules.

## Technical implementation

- [Technical implementation index](xdremux/README.en.md): stable implementation contracts.
- [ReverseKey1Ensemble model card](../Models/ReverseKey1Ensemble.model-card.en.md): optional research model contract.

## Historical records

The following files are evidence records. They describe a specific repository state or a specific experiment. They are not current product specifications.

- ISO conformance audit, 2026-05-11: [current-language summary](xdremux/iso-conformance-audit-20260511.summary.en.md) | [original record](xdremux/iso-conformance-audit-20260511.md)
- Encoding quality and size audit, 2026-07-18: [current-language summary](validation/encoding-quality-pareto-20260718.summary.en.md) | [original record](validation/encoding-quality-pareto-20260718.md)
- Vendor Live Photo geometry evidence: [current-language summary](validation/vendor-live-photo-geometry.summary.en.md) | [original record](validation/vendor-live-photo-geometry.md)

Use the current documents above for product behavior. Do not infer a current guarantee from an old path, measurement, or implementation note in a historical record.

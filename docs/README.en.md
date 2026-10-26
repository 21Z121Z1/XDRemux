# Documentation

English | [简体中文](README.md)

Use the [project README](../README.en.md) to build the CLI and run a first conversion. This index separates current product instructions from research and historical evidence.

## Use XDRemux

| Document | Content |
| --- | --- |
| [CLI reference](cli.en.md) | Commands, options, defaults, output paths, errors, and reuse conditions. |
| [Apple features](apple-features.en.md) | Portrait, Photographic Styles, Texture Styles, source requirements, and native limits. |
| [Cinematic video](cinematic.en.md) | Pending metadata, native preprocessing, and experimental video support. |
| [Supported devices](supported-devices.en.md) | Reported source compatibility and available fixture evidence. |
| [Live Photo integration](liblivephoto-integration.en.md) | Motion Photo conversion, media preservation, pairing, and PVT output. |

## Develop and verify

| Document | Content |
| --- | --- |
| [Architecture](architecture.en.md) | Implementation owners and media contracts. |
| [Development and builds](development.en.md) | Rust workspace, Apple adapter, app, and packaging. |
| [Testing policy](quality/testing.en.md) | Required evidence and the limits of each test layer. |
| [Regression and sample checks](quality/evals.en.md) | Reusable checks and real-file verification. |
| [Output policy](quality/logging.en.md) | Standard output, standard error, JSON, and error messages. |
| [Validation runbook](validation/README.en.md) | Verification plans and receipts bound to a committed source state. |
| [Test suite guide](../tests/README.en.md) | Test owners and commands. |
| [Fixture guide](../fixtures/README.en.md) | Versioned media and identity rules. |
| [Script index](../scripts/README.en.md) | Build, validation, performance, and research commands. |
| [Agent guide](../AGENTS.md) | Repository rules and project pstack skills. |
| [Technical writing guide](style-guide.en.md) | ASD-STE100, Chinese writing rules, terminology, and bilingual review. |

Current bilingual documents use English as their canonical source. Both languages must preserve the same behavior and limitations.

## Read or reproduce research

Research tools do not define the public product contract. Each record states what its inputs, measurements, or native observations establish.

- [OPPO style research](../research/oppo_styles/README.en.md) describes model training, inference, and evidence limits.
- [Research methodology](../research/oppo_styles/methodology.en.md) defines data selection, losses, and evaluation.
- [ReverseKey1Ensemble model card](../research/oppo_styles/models/ReverseKey1Ensemble.model-card.en.md) describes the research model and binary layout.
- [UniversalPhotographicStyleStateNet model card](../research/oppo_styles/models/UniversalPhotographicStyleStateNet.model-card.en.md) describes the joint research predictor.
- [Texture Styles research](../research/texture_styles/README.en.md) records texture metadata and native behavior.
- [Video style research](../research/video_styles/README.md) examines video metadata carriers and consumer evidence.
- [Palette research](../research/palette/README.en.md) describes source-grounded scene classification experiments.

## Read historical evidence

These records describe a specific version, experiment, or source state. Their measurements are not current product guarantees.

| Record | Reading options |
| --- | --- |
| ISO conformance audit, 2026-05-11 | [Summary](history/iso-conformance-audit-20260511.summary.en.md), [original record](history/iso-conformance-audit-20260511.md). |
| Encoding quality and size, 2026-07-18 | [Summary](validation/encoding-quality-pareto-20260718.summary.en.md), [original record](validation/encoding-quality-pareto-20260718.md). |
| Vendor Live Photo geometry | [Summary](validation/vendor-live-photo-geometry.summary.en.md), [original record](validation/vendor-live-photo-geometry.md). |
| HDR Log multi-sample check, 2026-10-14 | [Validation record](validation/20261014-hdr-log-multisample.en.md). |
| HDR Log thumbnail check, 2026-10-14 | [Validation record](validation/20261014-hdr-log-thumbnail.en.md). |
| Version 1.4 | [Release record](releases/v1.4.md). |

Use current references for current behavior. Retain historical paths, values, and conclusions when reading an older result.

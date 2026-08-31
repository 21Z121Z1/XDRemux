# XDRemux System Architecture and Agent Map

English | [简体中文](architecture.md)

This document defines the architectural model that agents and maintainers use to reason about XDRemux as one system.

Use this document before you read implementation details. Use the [transition roadmap](roadmap.en.md) for current migration work. Use [AGENTS.md](../AGENTS.md) for the acceptance contract.

## System model

XDRemux is not a Swift project, a Python project, a Rust project, or a Photographic Styles research project. Those are implementation and research lanes inside one media-conversion system.

The stable architecture is defined by media semantics, capability boundaries, product contracts, and evidence. A programming language is not an architectural layer.

The system has two orthogonal planes:

- the **product plane**, which turns an input asset and a request into a validated output asset;
- the **control plane**, which records contracts, fixtures, conformance evidence, research provenance, and acceptance state.

Research can propose new behavior to the product plane. Research does not own product policy.

## Abstraction tower

Read the system from the bottom up. Higher layers can depend on lower layers. Lower layers must not depend on product shells or research policy.

### Layer 0: evidence and external contracts

This layer answers: **what must remain true?**

It contains:

- ISO/TS 21496-1 and other external format contracts;
- public fixtures and fixture hashes;
- private or device-only evidence where public fixtures are not possible;
- cross-implementation conformance vectors;
- regression tests for known failures;
- device and consumer validation receipts.

Evidence does not decide implementation structure. It constrains all implementations.

### Layer 1: binary and format primitives

This layer answers: **what bytes are present and how are they represented?**

Responsibilities include:

- endian-safe byte access;
- FourCC and ISO-BMFF box models;
- hardened parsing and construction;
- Exif/TIFF parsing and orientation;
- JPEG and HEVC structural parsing;
- bounds and overflow validation.

The active Rust implementation is centered on `xdremux-format`. The Swift v1.4 implementation contains equivalent responsibilities under `Sources/XDRemuxCore/HEIF/`, metadata helpers, and adjacent format code.

This layer must not know about CLI options, Apple Photos behavior, model selection, or batch publication policy.

### Layer 2: normalized media semantics

This layer answers: **what does the asset mean?**

Responsibilities include:

- OPPO/OnePlus/realme private HDR metadata interpretation;
- Gain Map parameters and EDR semantics;
- source container resource extraction;
- Motion Photo topology, timing, payload, and vendor metadata;
- photo-asset classification;
- normalized source profiles used by planning.

The Rust transition currently separates these concerns across `xdremux-metadata`, `xdremux-hdr`, `xdremux-container`, `xdremux-motion-photo`, and `xdremux-classification`.

Semantic models must prefer normalized facts over raw vendor fields. Vendor-specific parsing can exist at the edge of this layer, but higher layers should consume normalized models when a stable model exists.

### Layer 3: deterministic planning and policy

This layer answers: **given facts, a request, and available capabilities, what should the system do?**

The target owner is `xdremux-engine`.

Planning must be deterministic and side-effect free. It can consume:

- normalized source facts;
- user intent;
- product policy;
- a capability inventory.

It must produce an explicit plan that records effective choices and required operations.

The Rust engine already uses operation-scoped capability facts instead of a monolithic platform backend. Preserve that rule. One conversion can compose capabilities from multiple adapters.

Do not hide policy inside a codec wrapper, native helper, CLI parser, or model predictor.

### Layer 4: execution adapters

This layer answers: **which concrete implementation performs a required operation?**

Adapters own side effects and platform dependencies. Examples include:

- raster decoding;
- HEVC Gain Map tile encoding;
- RAW processing;
- ImageIO or Photos consumer validation;
- Photographic Styles behavior;
- Apple Portrait behavior.

The engine depends on narrow operation contracts. Concrete adapters depend on the engine contracts, not the reverse.

Do not create one universal `Backend` object that combines unrelated operations. Do not create a new crate only to mirror an old Swift directory. Create a boundary only when the capability has a stable contract, independent tests, and a reason to evolve separately.

### Layer 5: asset transformation and publication

This layer answers: **how is the planned media result materialized safely?**

Responsibilities include:

- HEIF output construction;
- compressed-sample passthrough where required;
- Motion Photo to Live Photo transformation;
- Live Photo timing and shared asset identity;
- pair publication, provenance, collision handling, and crash recovery;
- output validation before publication is considered successful.

Publication is part of correctness. A structurally valid temporary output is not a successful product result until the publication contract is satisfied.

### Layer 6: product composition

This layer answers: **how does a user or application invoke the system?**

Responsibilities include:

- CLI parsing and localized terminal output;
- library API composition roots;
- batch orchestration;
- the macOS app;
- progress and structured events;
- product defaults.

Product shells must translate user intent into engine requests and adapter composition. They must not become alternate owners of media semantics.

## Cross-cutting control plane

The control plane is not a higher product layer. It crosses every layer.

### Acceptance and conformance

`AGENTS.md`, `scripts/agent_completion_gate.py`, CI, fixtures, and conformance oracles define what evidence is required for a claim.

A migration is not complete because the Rust code looks equivalent to Swift. It is complete only when the relevant behavioral contracts have independent evidence.

Cross-language comparison is useful during migration, but the old implementation is not automatically the specification. When an old implementation conflicts with an external standard, current product contract, or stronger evidence, resolve the conflict explicitly.

### Research plane

`Models/`, model cards, training scripts, and research branches form a research plane beside the product architecture.

A research model can produce a **candidate** or **proposal**. It must not silently become the source of product truth.

For Photographic Styles, keep these boundaries explicit:

1. input and provenance;
2. teacher or label source;
3. model prediction;
4. uncertainty or gate decision;
5. native consumer or renderer evidence;
6. product promotion decision.

A lower offline loss is not sufficient evidence for product promotion.

## Stable capability vocabulary

Use these capability identifiers in plans, issues, and architecture discussions when they make the scope clearer. They describe ownership, not filenames.

| Capability | Architectural owner | Typical evidence |
| --- | --- | --- |
| `format.binary` | Layer 1 | parser/constructor vectors, malformed-input tests |
| `format.heif` | Layers 1 and 5 | structural conformance, real output inspection |
| `metadata.vendor-hdr` | Layer 2 | fixture extraction, cross-implementation vectors |
| `hdr.gain-map` | Layer 2 | formula parity, image/metadata validation |
| `media.motion-photo` | Layer 2 | vendor fixtures, topology/timing tests |
| `media.live-photo` | Layer 5 | pair identity, timing, PhotoKit/device evidence |
| `asset.classification` | Layer 2 | classification contract fixtures |
| `engine.plan` | Layer 3 | deterministic request/analysis/plan vectors |
| `adapter.codec` | Layer 4 | codec capability and payload tests |
| `adapter.apple.styles` | Layer 4 | native consumer/renderer and device evidence |
| `adapter.apple.portrait` | Layer 4 | native consumer and device evidence |
| `product.cli` | Layer 6 | parser, routing, output, exit-contract tests |
| `product.app` | Layer 6 | app integration and UI workflow evidence |
| `research.styles-model` | Research plane | data provenance, held-out/OOD, consumer A/B |

Add a new identifier only when a stable responsibility cannot fit an existing capability.

## Current implementation lanes

### `main`

`main` is the released v1.4 product and the current public behavioral reference for the Swift and Python line.

v1.4 is the final release line that ships both Swift and Python implementations. New product development is moving to Rust.

Maintain `main` when a released behavior, safety property, documentation contract, or migration oracle needs correction. Do not add large new architecture only to keep the old implementation feature-complete with the rewrite.

### `feat/rust-xdremux-format`

This branch name is historical. Its scope is now the Rust rewrite, not only format parsing.

The branch contains the Rust format, metadata, HDR, container, HEIF, Motion Photo, classification, and engine layers plus cross-implementation conformance tools.

Treat this branch as the active migration implementation line. Treat `main` and v1.4 evidence as reference inputs to migration, not as a directory-by-directory porting checklist.

### `codex/reverse-key1-oppo-solver`

This branch is a Photographic Styles research line.

It contains universal style-model training, optional RAW-linear and Gain Map modalities, Core ML export/inference work, data-provenance controls, and evaluation tooling.

Treat outputs from this branch as research candidates until the promotion gates in the [transition roadmap](roadmap.en.md) pass.

## Source-of-truth rules

Use the narrowest authoritative source for each question:

1. For released user behavior, use the current release contract, current public documentation, implementation, and matching evidence.
2. For architectural ownership, use this document and the current code boundaries.
3. For an active branch implementation fact, inspect that branch and compare it with its intended base.
4. For a behavioral invariant, prefer external standards, fixtures, conformance tests, and device evidence over comments or historical implementation shape.
5. For research claims, use the model card, dataset provenance, evaluation code, and current measured artifacts.
6. Treat dated audits, old PR descriptions, and old experiment notes as historical records unless a current document explicitly adopts their conclusion.

No long-lived branch may be the only place where a stable system contract is documented.

## Agent bootstrap protocol

For a substantial task, an agent must establish a small working set before broad code search:

1. Record the current branch, exact `HEAD`, intended base, and clean/dirty state.
2. Read this document.
3. Read the [transition roadmap](roadmap.en.md) when the task touches migration or research.
4. Identify the affected capability identifiers and architectural layers.
5. Read only the owning modules, neighboring tests, relevant fixtures, and current normative documents.
6. Compare the active branch with its intended base before assuming that either side contains all current knowledge.
7. Write explicit acceptance criteria before implementation when the task is cross-layer or changes a contract.
8. Use the acceptance sequence in `AGENTS.md` before claiming completion.

This protocol is designed to reduce repository-wide scanning without reducing correctness.

## Dependency rules

Keep these rules during the Rust transition and later maintenance:

- format primitives do not depend on media policy;
- normalized media semantics do not depend on CLI, app, or research code;
- planning does not perform I/O or call concrete platform frameworks;
- adapters do not own product policy;
- publication does not infer provenance from filename shape alone;
- product shells do not reimplement parsing or media semantics;
- research code does not silently set production defaults;
- validation code can observe all layers but must not become a hidden production dependency;
- cross-language oracles are migration tools, not permanent architectural coupling unless a release contract explicitly requires both implementations.

When a proposed change violates one of these rules, prefer moving the responsibility to its owning layer instead of adding another special case.

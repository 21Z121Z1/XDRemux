# XDRemux System Architecture and Agent Map

English | [简体中文](architecture.md)

This document defines the stable architectural model that agents and maintainers use to reason about XDRemux as one system.

Use this document for ownership and dependency rules. Use [`agent-map.json`](agent-map.json) for machine-readable routing. Use the [transition roadmap](roadmap.en.md) for promotion and retirement rules. Use [AGENTS.md](../AGENTS.md) for the operating and acceptance contract.

## System model

XDRemux is not a Swift project, a Python project, a Rust project, or a Photographic Styles research project. Those are implementation and research lanes inside one media-conversion system.

The stable architecture is defined by media semantics, capability boundaries, product contracts, and evidence. A programming language, branch, directory, crate, or workflow name is not an architectural layer.

The system has two orthogonal planes:

- the **product plane**, which turns an input asset and a request into a validated output asset;
- the **control plane**, which records contracts, routing metadata, fixtures, conformance evidence, research provenance, promotion state, and acceptance state.

Research can propose new behavior to the product plane. Research does not own product policy.

## Stable knowledge and live state

Keep stable knowledge separate from facts that change on every commit.

Stable knowledge belongs in normative documents, tests, model cards, or `docs/agent-map.json`. Examples are capability identifiers, layer ownership, branch roles, invariants, evidence requirements, and promotion rules.

Live state must be derived from the repository. Examples are the current `HEAD`, ahead/behind counts, workspace membership, changed paths, workflow results, and a diagnostic probe's latest outcome.

Do not copy live state into architecture documents when Git, manifests, code, or CI can answer the question directly. Use:

```bash
python3 scripts/agent_context.py status
python3 scripts/agent_context.py capability engine.plan
```

This separation prevents a correct architecture document from becoming stale merely because an implementation branch moves.

## Abstraction tower

Read the product plane from the bottom up. Higher layers can depend on lower layers. Lower layers must not depend on product shells or research policy.

### Layer 0: evidence and external contracts

This layer answers: **what must remain true?**

It contains:

- ISO/TS 21496-1 and other external format contracts;
- public fixtures and fixture hashes;
- private or device-only evidence where public fixtures are not possible;
- cross-implementation conformance vectors;
- regression tests for known failures;
- consumer and device validation receipts.

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

The active Rust implementation is centered on `xdremux-format` and `xdremux-heif`. The Swift v1.4 reference contains equivalent responsibilities under `Sources/XDRemuxCore/HEIF/`, metadata helpers, and adjacent format code.

This layer must not know about CLI options, Apple Photos behavior, model selection, or batch publication policy.

### Layer 2: normalized media semantics

This layer answers: **what does the asset mean?**

Responsibilities include:

- OPPO/OnePlus/realme private HDR metadata interpretation;
- Gain Map parameters and EDR semantics;
- vendor resource extraction;
- Motion Photo topology, timing, payload, and vendor metadata;
- photo-asset classification;
- normalized source profiles used by planning.

Rust ownership is split by semantic responsibility rather than by old Swift directory shape. Current owners include `xdremux-metadata`, `xdremux-hdr`, `xdremux-container`, `xdremux-motion-photo`, and `xdremux-classification`.

Semantic models must prefer normalized facts over raw vendor fields. Vendor-specific parsing can exist at the edge of this layer, but higher layers should consume normalized models when a stable model exists.

### Layer 3: deterministic planning and policy

This layer answers: **given facts, a request, and available capabilities, what should the system do?**

The Rust owner is `xdremux-engine`.

Planning must be deterministic and side-effect free. It can consume:

- normalized source facts;
- user intent;
- product policy;
- a capability inventory.

It must produce an explicit plan that records effective choices and required operations.

The engine uses operation-scoped capability facts instead of a monolithic platform backend. One conversion can compose capabilities from multiple adapters. Capability discovery reports facts; it does not choose product policy.

Stable representation types from lower layers may be referenced by the planner when they are genuinely part of the contract. This must not pull parsing, I/O, or platform behavior upward into planning.

Do not hide policy inside a codec wrapper, native helper, CLI parser, or model predictor.

### Layer 4: execution adapters

This layer answers: **which concrete implementation performs a required operation?**

Adapters own side effects, external libraries, and platform dependencies. Examples include:

- raster decoding;
- HEVC Gain Map tile encoding;
- RAW processing;
- ImageIO or Photos consumer validation;
- Photographic Styles behavior;
- Apple Portrait behavior.

The engine depends on narrow operation contracts. Concrete adapters depend on those contracts, not the reverse.

`xdremux-codec` is the first concrete Rust adapter boundary. It implements engine codec ports through a portable libheif provider. The existence of the crate is not proof that the capability has passed its promotion evidence; current provider support and CI state must be read from the active Rust branch.

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

Product shells translate user intent into engine requests and adapter composition. They must not become alternate owners of media semantics.

## Cross-cutting control plane

The control plane is not a higher product layer. It crosses every layer.

### Routing metadata

`docs/agent-map.json` is the machine-readable routing index for stable capability identifiers, owner hints, evidence categories, and long-lived branch roles.

The JSON file is not a second architecture specification. Human semantics and dependency rules remain in this document. CI must keep the routing identifiers synchronized with the architecture.

### Acceptance and conformance

`AGENTS.md`, `scripts/agent_completion_gate.py`, CI, fixtures, and conformance oracles define what evidence is required for a claim.

Keep both **evidence class** and **evidence role** explicit. A regression test and a device test are different evidence classes. A required merge gate, a capability promotion gate, and a diagnostic probe are different evidence roles.

A diagnostic probe can patch a checkout or inspect an environment to discover facts. It is not acceptance or promotion evidence until the relevant behavior is turned into a reproducible contract check. See the [validation runbook](validation/README.en.md).

A migration is not complete because the Rust code looks equivalent to Swift or because a diagnostic workflow turns green. It is complete only when the relevant behavioral contracts have the required independent evidence.

Cross-language comparison is useful during migration, but the old implementation is not automatically the specification. When an old implementation conflicts with an external standard, current product contract, or stronger evidence, resolve the conflict explicitly.

### Research plane

`Models/`, model cards, training/evaluation scripts, and research branches form a research plane beside the product architecture.

A research model can produce a **candidate** or **proposal**. It must not silently become the source of product truth.

For Photographic Styles, keep these boundaries explicit:

1. input and provenance;
2. teacher or label source;
3. model prediction;
4. uncertainty or gate decision;
5. native consumer or renderer evidence;
6. product promotion decision.

A lower offline loss is not sufficient evidence for product promotion.

Stable model contracts belong in model cards. Stable training, dataset, and evaluation protocols belong in dedicated research documentation on the research line until they are promoted. Do not turn the general development guide into an experiment log.

### Execution plans

Use `docs/exec-plans/` only for work that must survive one agent session or one PR. Plans record resumable facts, decisions, evidence, blockers, and next actions. They do not replace architecture and they must not contain private chain-of-thought.

If an execution plan discovers a stable repository-wide rule, promote the rule into its normative owner rather than leaving it trapped in a completed plan.

## Stable capability vocabulary

Use these capability identifiers in plans, issues, PRs, and architecture discussions when they make scope clearer. The same identifiers are mirrored in `docs/agent-map.json` for machine routing.

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
| `adapter.codec` | Layer 4 | capability advertisement, payload tests, provider round trips |
| `adapter.apple.styles` | Layer 4 | native consumer/renderer and device evidence |
| `adapter.apple.portrait` | Layer 4 | native consumer and device evidence |
| `product.cli` | Layer 6 | parser, routing, output, exit-contract tests |
| `product.app` | Layer 6 | app integration and UI workflow evidence |
| `research.styles-model` | Research plane | data provenance, held-out/OOD, consumer A/B |

Add a new identifier only when a stable responsibility cannot fit an existing capability.

## Implementation lanes and branch roles

Branch names are routing metadata, not architecture. Long-lived branch roles are machine-readable in `docs/agent-map.json`.

### `main`

`main` is the released v1.4 product and the current public behavioral reference for the Swift and Python line. It is also the intended home for shared architecture and validation contracts.

v1.4 is the final release line that ships both Swift and Python implementations. New product development is moving to Rust.

Maintain `main` when a released behavior, safety property, shared documentation contract, or migration oracle needs correction. Do not add large new architecture only to keep the old implementation feature-complete with the rewrite.

### `feat/rust-xdremux-format`

This branch name is historical. Its scope is now the Rust rewrite, not only format parsing.

Treat it as the active migration implementation line. Inspect its current `Cargo.toml`, code, tests, and workflow results for live implementation state; do not use a copied crate list in a roadmap as the authority.

Treat `main` and v1.4 evidence as reference inputs to migration, not as a directory-by-directory porting checklist.

### `codex/reverse-key1-oppo-solver`

This branch is a Photographic Styles research line.

It contains model training, optional RAW-linear and Gain Map modalities, Core ML work, provenance controls, and evaluation tooling. Its model cards and measured artifacts are authoritative for research facts on that branch, not for production defaults.

Treat outputs from this branch as research candidates until the promotion gates in the [transition roadmap](roadmap.en.md) pass.

## Source-of-truth rules

Use the narrowest authoritative source for each question:

1. For released user behavior, use the current release contract, current public documentation, implementation, and matching evidence.
2. For stable architectural ownership, use this document.
3. For capability routing and long-lived branch roles, use `docs/agent-map.json`.
4. For an active branch implementation fact, inspect that branch and compare it with its intended base.
5. For current branch/HEAD/divergence, derive Git state with `scripts/agent_context.py` or Git directly.
6. For a behavioral invariant, prefer external standards, fixtures, conformance tests, and device evidence over comments or historical implementation shape.
7. For research claims, use model cards, dataset provenance, evaluation code, and current measured artifacts.
8. Treat dated audits, old PR descriptions, completed plans, and old experiment notes as historical evidence unless a current document explicitly adopts their conclusion.

No long-lived branch may be the only place where a stable system contract is documented.

## Agent bootstrap protocol

For a substantial task, establish a small working set before broad code search:

1. Run `python3 scripts/agent_context.py status`. Pass `--base` if the branch is not registered.
2. Identify the affected capability identifiers and layers. Use `python3 scripts/agent_context.py capability <id>` for routing.
3. Read only the owning modules, neighboring tests, relevant fixtures, and current normative documents.
4. Compare the active branch with its intended base before assuming either side contains all current knowledge.
5. Read the roadmap for migration/research state, not for volatile Git facts.
6. Create an execution plan only when the work meets the criteria in `docs/exec-plans/README.en.md`.
7. Write explicit acceptance criteria before implementation when the task is cross-layer or changes a contract.
8. Use the acceptance sequence in `AGENTS.md` before claiming completion.

This protocol reduces repository-wide scanning without reducing correctness.

## Dependency and instruction rules

Keep these rules during the Rust transition and later maintenance:

- format primitives do not depend on media policy;
- normalized media semantics do not depend on CLI, app, or research code;
- planning does not perform I/O or call concrete platform frameworks;
- adapters depend on operation contracts and do not own product policy;
- publication does not infer provenance from filename shape alone;
- product shells do not reimplement parsing or media semantics;
- research code does not silently set production defaults;
- validation code can observe all layers but must not become a hidden production dependency;
- cross-language oracles are migration tools, not permanent architectural coupling unless a release contract explicitly requires both implementations;
- root instructions contain universal rules only;
- add path-specific or nested agent instructions only for true local invariants, and never duplicate a repository-wide rule into several instruction files.

When a proposed change violates one of these rules, prefer moving the responsibility to its owning layer instead of adding another special case.

# XDRemux Transition Roadmap

English | [简体中文](roadmap.md)

This document defines the current migration plan from the released v1.4 Swift/Python line to the Rust product line. It also defines how the Photographic Styles research line can enter the product.

Use the [system architecture](architecture.en.md) for ownership and dependency rules. This roadmap records transition state and promotion gates.

## Goal

The goal is not to translate every Swift or Python function into Rust.

The goal is to preserve or improve the verified XDRemux product contracts while moving stable media semantics and product orchestration into a smaller, more explicit Rust architecture.

The migration must reduce three forms of ambiguity:

- which layer owns a behavior;
- which implementation is authoritative for a claim;
- which evidence is required before a capability is promoted.

## Current branch roles

| Branch | Role | Canonical for | Not canonical for | Retirement condition |
| --- | --- | --- | --- | --- |
| `main` | released v1.4 maintenance/reference | v1.4 public behavior and release artifacts | future Rust architecture | Rust release supersedes the relevant product contracts and v1.4 maintenance is no longer needed |
| `feat/rust-xdremux-format` | active Rust rewrite | current Rust implementation work | released user behavior until a Rust release exists | rename or merge into the Rust release line after the rewrite reaches product readiness |
| `codex/reverse-key1-oppo-solver` | Photographic Styles research | current research implementation and experiments on that branch | production defaults and released quality claims | useful research is promoted through explicit gates or archived as historical evidence |

Branch names are references, not architecture. The Rust branch name is already narrower than its real scope.

At the start of work on any long-lived branch, compare it with its intended base. Do not copy ahead/behind counts into normative documents because they become stale after every commit.

## Current Rust foundation

The Rust workspace currently contains:

- `xdremux-format`;
- `xdremux-metadata`;
- `xdremux-hdr`;
- `xdremux-container`;
- `xdremux-heif`;
- `xdremux-motion-photo`;
- `xdremux-classification`;
- `xdremux-engine`.

The branch also contains Swift/Python-to-Rust conformance oracles and focused Rust CI workflows.

This is already beyond a format-parser experiment. The next work should complete the architecture around this foundation instead of adding more direct function ports.

## Migration invariant

For each capability that moves to Rust, record these four items:

1. the normalized contract;
2. the old implementation or external evidence used as an oracle;
3. the Rust owner;
4. the promotion evidence.

If one of these is missing, the migration is incomplete even when the Rust code compiles.

## Phase 1: freeze the v1.4 behavioral contract

Purpose: make the released Swift/Python line a bounded reference instead of an indefinitely evolving competitor.

Required work:

- keep public documentation accurate for v1.4;
- preserve the real Motion Photo fixture corpus and its hashes;
- preserve conversion-safety and publication regressions;
- preserve the exact behaviors that Rust conformance checks need;
- correct released safety defects when necessary;
- avoid unrelated large feature development on the old line.

Exit criteria:

- each Rust migration area can point to a stable test, fixture, standard, or current product contract;
- no active migration depends only on undocumented behavior in a long-lived branch.

## Phase 2: complete the pure Rust semantic core

Purpose: finish Layers 1 through 3 of the architecture before product-shell expansion.

The existing crates already cover most of the intended semantic partitions. New work should focus on missing contracts and composition, not crate count.

Required properties:

- parsers fail closed on malformed bounds and lengths;
- vendor metadata is converted into normalized semantic models;
- Gain Map semantics are independent from container-writing side effects;
- Motion Photo parsing produces stable topology/timing/payload models;
- classification consumes normalized asset facts;
- `xdremux-engine` produces deterministic plans from source facts, requests, and capability inventory;
- planning remains free of platform I/O.

Exit criteria:

- every migrated semantic capability has targeted Rust tests;
- cross-implementation vectors pass where v1.4 is a useful oracle;
- external-standard tests exist where the old implementation is not sufficient as a specification;
- no required Rust semantic path depends on Swift or Python at runtime.

## Phase 3: implement execution adapters without rebuilding a monolith

Purpose: make the plans executable while keeping platform/library operations replaceable.

The current engine already defines operation-scoped ports for raster decoding, Gain Map tile encoding, RAW processing, consumer validation, Photographic Styles, and Portrait. Keep that model.

Required work:

- provide concrete codec and platform adapters for operations required by standard HDR conversion;
- define request/output types only when their stable boundary is understood;
- keep capability discovery factual and separate from policy;
- test each adapter through its operation contract;
- keep native or closed-framework code outside the pure semantic core.

Do not introduce a universal platform backend. Do not make one adapter mandatory for unrelated operations.

Exit criteria:

- standard HDR plans can be executed end to end through explicit adapters;
- missing capabilities fail with an explicit planner or composition error;
- adapter-specific failures do not alter engine policy silently.

## Phase 4: restore complete asset and publication semantics

Purpose: move from a correct still-image core to the complete XDRemux asset model.

Required work includes the v1.4 behaviors that are easy to lose in a rewrite:

- Motion Photo cover-frame timing;
- compressed video and audio passthrough where required;
- Apple Live Photo shared asset identity;
- deterministic output naming rules;
- pair provenance;
- collision handling;
- crash-recoverable pair publication;
- batch resume rules;
- source preservation and destructive-operation safety.

These are product correctness contracts, not CLI conveniences.

Exit criteria:

- public Motion Photo fixtures pass the Rust path;
- Live Photo pair identity and timing match the contract;
- pair publication regressions cover crash/collision/provenance cases;
- macOS PhotoKit or equivalent integration evidence validates generated pairs where the claim requires it.

## Phase 5: add Apple-specific product adapters

Purpose: keep Apple-only behavior at explicit outgoing capability boundaries.

Photographic Styles and Portrait must not move into the pure engine as platform assumptions.

Required work:

- define concrete Apple adapter composition for the engine ports;
- preserve runtime ABI negotiation and fail-closed behavior for private frameworks where still required;
- preserve consumer validation as a separate operation from container construction;
- keep device-dependent claims behind device evidence.

Exit criteria:

- Apple feature requests are explicit engine requirements;
- a non-Apple composition can build and use the standard core without Apple frameworks;
- Apple consumer and device evidence pass for each promoted feature.

## Phase 6: create the Rust product shell

Purpose: make Rust the product line instead of a set of conformance crates.

Required work:

- expose a stable library composition root;
- add the supported CLI surface only after engine request models are stable;
- map structured engine events to user output without putting terminal policy into the core;
- preserve current output-safety semantics;
- integrate the macOS app through a narrow library/FFI boundary if the app remains SwiftUI;
- document packaging and release artifacts.

Do not begin by duplicating every old CLI option. Promote options that correspond to supported engine contracts. Retire research-only or obsolete controls instead of automatically carrying them forward.

Exit criteria:

- standard conversion, Motion Photo/Live Photo, classification, and selected Apple features are reachable through the intended product entry points;
- CLI and app do not reimplement media policy;
- release packaging is reproducible and CI-bound to exact `HEAD`.

## Phase 7: Rust release promotion

A Rust release can supersede v1.4 only when the exact release candidate satisfies all applicable evidence classes.

Minimum release evidence:

- format/parser regression suite;
- Rust semantic unit and integration tests;
- cross-implementation conformance vectors where useful;
- public Motion Photo real-fixture gates;
- standard HDR real-fixture conversion and container validation;
- output-safety and publication regressions;
- CLI/product integration tests;
- Apple consumer/device evidence for every Apple feature included in the release;
- exact-HEAD completion receipt;
- required GitHub Actions and CodeQL checks on the release candidate.

A feature that cannot meet its evidence gate must be excluded, marked experimental, or explicitly scoped. It must not be silently accepted because the rest of the release is green.

## Photographic Styles research promotion

The research branch follows a separate promotion ladder. Model accuracy work can proceed in parallel with the Rust rewrite, but product promotion is gated independently.

### Research state

The current research line includes:

- a primary-image model path;
- optional RAW-derived linear RGB input;
- optional Gain Map input;
- explicit modality masks and modality dropout;
- public synthetic/content-domain pretraining;
- private native/teacher-labelled training support;
- Core ML export and Swift research integration;
- held-out, OOD, cascade, and consumer-oriented evaluation tools.

These capabilities are research infrastructure. They are not equivalent to production validation.

### Promotion gates

A model or learned component must pass these gates in order:

1. **Data provenance**: every training/evaluation input has a known source, license or private-data status, identity hash, and label/teacher provenance.
2. **Leakage control**: calibration, training, held-out, and final locked sets do not share source sessions or derived copies in ways that invalidate the metric.
3. **Primary-only robustness**: optional modalities improve results when present but their absence does not collapse the required ordinary-image path.
4. **Held-out and OOD**: the candidate beats the current accepted baseline on predefined metrics and important device/content strata.
5. **Consumer correlation**: lower parameter loss also improves the real renderer/consumer response that the product cares about.
6. **Bounded uncertainty**: the product has a measurable reject/fallback rule for cases outside the model's supported envelope.
7. **End-to-end product evidence**: actual generated assets pass container, native consumer, and device tests.
8. **Operational budget**: runtime, memory, model size, and failure behavior fit the product target.

Only after these gates pass can a model move from `research.styles-model` to an adapter or engine-visible production capability.

The engine should consume the promoted capability through a stable adapter contract. It should not import training code or know how the model was trained.

## Branch lifecycle rules

Every long-lived branch must have four explicit facts in a current document or PR description:

- role;
- intended base;
- promotion gate;
- retirement condition.

Before deleting a branch:

1. compare it with its intended destination;
2. identify commits or contracts that do not exist elsewhere;
3. promote useful current knowledge into code, tests, model cards, or normative documents;
4. preserve dated experimental evidence as historical records when it remains useful;
5. delete only after no stable contract depends on branch-only knowledge.

A merged implementation is not enough if the reasoning, acceptance boundary, or provenance needed to maintain it exists only in an old PR or chat transcript.

## Agent execution pattern

For substantial migration work, use this compact task ledger in the PR description or working notes:

- **Target capability**: one or more identifiers from `architecture.en.md`.
- **Base and branch**: exact refs and merge base.
- **Current owner**: source files/crates that own the behavior today.
- **Invariant**: the behavior that must remain true.
- **Oracle/evidence**: standards, fixtures, v1.4 behavior, device result, or measured research baseline.
- **Change boundary**: layers allowed to change.
- **Acceptance checks**: commands/workflows required for promotion.
- **Residual gaps**: facts not proven by the available environment.

Do not create repository files only to store transient chain-of-thought or session notes. Persist only decisions, contracts, provenance, reusable evidence, and plans that another maintainer or agent must recover later.

# XDRemux Transition Roadmap

English | [简体中文](roadmap.md)

This document defines the transition from the released v1.4 Swift/Python line to the Rust product line and the promotion path for Photographic Styles research.

Use the [system architecture](architecture.en.md) for stable ownership and dependency rules. Use [`agent-map.json`](agent-map.json) for machine-readable branch/capability routing. This roadmap records stages and promotion gates, not volatile Git state.

## Goal

The goal is not to translate every Swift or Python function into Rust.

The goal is to preserve or improve verified XDRemux product contracts while moving stable media semantics and product orchestration into a smaller, more explicit Rust architecture.

The transition must reduce four forms of ambiguity:

- which layer owns a behavior;
- which implementation or evidence source is authoritative for a claim;
- which evidence is required before a capability is promoted;
- which current facts must be derived live instead of copied into prose.

## Dynamic-state rule

Do not maintain current `HEAD`, ahead/behind counts, complete workspace membership, changed-file lists, or current workflow results in this roadmap.

Derive them from Git, code, manifests, and CI:

```bash
python3 scripts/agent_context.py status
python3 scripts/agent_context.py capability adapter.codec
```

On the Rust branch, its `Cargo.toml` is authoritative for current workspace membership. A crate existing in the workspace means an implementation boundary exists; it does not mean its capability has passed promotion evidence.

At the current architecture milestone, the semantic foundation reaches `xdremux-engine`, and `xdremux-codec` is the first concrete Layer 4 adapter boundary. Its providers must still earn capability promotion through their operation contracts and real runtime evidence.

## Branch lifecycle

The stable long-lived branch roles are stored in `docs/agent-map.json`:

- `main`: released v1.4 reference and shared control-plane destination;
- `feat/rust-xdremux-format`: active Rust migration implementation line;
- `codex/reverse-key1-oppo-solver`: Photographic Styles research line.

Branch names are references, not architecture. The Rust branch name is already narrower than its real scope.

Every long-lived branch must have four explicit facts: role, intended base, promotion gate, and retirement condition. Do not create another long-lived branch merely to represent an architecture layer.

## Migration invariant

For each capability that moves to Rust, record these four items in the PR, current contract, or active execution plan:

1. the normalized contract;
2. the old implementation or external evidence used as an oracle;
3. the Rust owner;
4. the promotion evidence.

If one item is missing, the migration is incomplete even when the Rust code compiles or a diagnostic workflow passes.

## Phase 1: freeze the v1.4 behavioral contract

Purpose: make the released Swift/Python line a bounded reference instead of an indefinitely evolving competitor.

Required work:

- keep v1.4 public documentation accurate;
- preserve the real Motion Photo fixture corpus and hashes;
- preserve conversion-safety and publication regressions;
- preserve behaviors needed as Rust conformance oracles;
- correct released safety defects when necessary;
- avoid unrelated large feature development on the old line.

Exit criteria:

- each Rust migration area can point to a stable test, fixture, standard, or current product contract;
- no active migration depends only on undocumented behavior in a long-lived branch.

## Phase 2: complete the pure Rust semantic core

Purpose: make Layers 1 through 3 explicit and independently testable before product-shell expansion.

Required properties:

- parsers fail closed on malformed bounds and lengths;
- vendor metadata becomes normalized semantic models;
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

## Phase 3: make plans executable through operation adapters

Purpose: execute engine plans without rebuilding a platform monolith.

The engine already defines operation-scoped ports for raster decoding, Gain Map tile encoding, RAW processing, consumer validation, Photographic Styles, and Portrait. `xdremux-codec` is the first concrete provider boundary and demonstrates the intended dependency direction: provider → engine port, not engine → concrete provider.

Required work:

- validate concrete codec/provider capabilities against real runtime behavior, not only advertised library support;
- provide the operations required by standard HDR conversion;
- compose providers at a product composition root rather than storing adapter instances inside planning facts;
- keep capability discovery factual and separate from policy;
- test each adapter through its operation contract;
- keep native or closed-framework code outside the pure semantic core.

Provider probes may temporarily patch a CI checkout to characterize dependency behavior. Such probes are diagnostic only until their finding is encoded in the actual implementation/test contract.

Exit criteria:

- standard HDR plans can be executed end to end through explicit adapters;
- required provider capabilities pass reproducible operation-contract tests in the supported runtime environment;
- missing capabilities fail with an explicit planner/composition error;
- adapter-specific failures do not alter engine policy silently;
- canonical provider tests do not depend on hidden CI-only source patches.

## Phase 4: restore complete asset and publication semantics

Purpose: move from a correct still-image path to the complete XDRemux asset model.

Required work includes v1.4 behaviors that are easy to lose in a rewrite:

- Motion Photo cover-frame timing;
- compressed video/audio passthrough where required;
- Apple Live Photo shared asset identity;
- deterministic output naming;
- pair provenance;
- collision handling;
- crash-recoverable pair publication;
- batch resume rules;
- source preservation and destructive-operation safety.

These are product correctness contracts, not CLI conveniences.

Exit criteria:

- public Motion Photo fixtures pass the Rust path;
- Live Photo pair identity and timing match the contract;
- pair-publication regressions cover crash/collision/provenance cases;
- PhotoKit or equivalent integration evidence validates generated pairs where the claim requires it.

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
- a non-Apple composition can build/use the standard core without Apple frameworks;
- Apple consumer and device evidence pass for each promoted feature.

## Phase 6: create the Rust product shell

Purpose: make Rust the product line rather than a set of conformance crates.

Required work:

- expose a stable library composition root;
- add the supported CLI surface after engine request models are stable;
- map structured engine events to user output without putting terminal policy into the core;
- preserve output-safety semantics;
- integrate the macOS app through a narrow library/FFI boundary if the app remains SwiftUI;
- document reproducible packaging and release artifacts.

Do not begin by duplicating every old CLI option. Promote options that correspond to supported engine contracts. Retire research-only or obsolete controls instead of automatically carrying them forward.

Exit criteria:

- standard conversion, Motion Photo/Live Photo, classification, and selected Apple features are reachable through intended product entry points;
- CLI and app do not reimplement media policy;
- release packaging is reproducible and CI-bound to exact `HEAD`.

## Phase 7: Rust release promotion

A Rust release can supersede v1.4 only when the exact release candidate satisfies every applicable evidence gate.

Minimum release evidence:

- format/parser regression suite;
- Rust semantic unit/integration tests;
- cross-implementation conformance vectors where useful;
- public Motion Photo real-fixture gates;
- standard HDR real-fixture conversion and container validation;
- output-safety and publication regressions;
- CLI/product integration tests;
- Apple consumer/device evidence for every included Apple feature;
- exact-HEAD completion receipt;
- required GitHub Actions and CodeQL checks on the release candidate.

A feature that cannot meet its evidence gate must be excluded, marked experimental, or explicitly scoped. It must not be silently accepted because the rest of the release is green.

## Verification control-plane convergence

During migration, focused workflows are useful because each Rust capability can evolve independently. They must not become the only way an agent can understand verification.

Move toward these properties as Rust becomes the product line:

- one documented repository-level Rust verification entry point for ordinary preflight;
- focused capability checks remain callable independently;
- workflow/check names communicate whether a check is required, promotion evidence, or diagnostic;
- diagnostic probes are not required merge checks unless converted into stable contract tests;
- the exact release/product gate composes the applicable capability checks rather than duplicating their logic.

The goal is not fewer workflow files by itself. The goal is one unambiguous answer to “what evidence proves this change?”

## Photographic Styles research promotion

The research line follows a separate promotion ladder. Model accuracy work can proceed in parallel with the Rust rewrite, but product promotion is gated independently.

Stable model contracts belong in model cards. Stable dataset/training/evaluation procedures belong in dedicated research documentation on the research line. Do not keep research protocol only in a general development guide or chat transcript.

A model or learned component must pass these gates in order:

1. **Data provenance**: every training/evaluation input has a known source, license/private-data status, identity hash, and label/teacher provenance.
2. **Leakage control**: calibration, training, held-out, and final locked sets do not share source sessions or derived copies in ways that invalidate the metric.
3. **Primary-only robustness**: optional modalities improve results when present but their absence does not collapse the ordinary-image path.
4. **Held-out and OOD**: the candidate beats the accepted baseline on predefined metrics and important device/content strata.
5. **Consumer correlation**: lower parameter loss also improves the real renderer/consumer response the product cares about.
6. **Bounded uncertainty**: the product has a measurable reject/fallback rule outside the supported envelope.
7. **End-to-end product evidence**: generated assets pass container, native consumer, and device tests.
8. **Operational budget**: runtime, memory, model size, and failure behavior fit the product target.

Only after these gates pass can a learned component move from `research.styles-model` to a production adapter capability.

The engine consumes the promoted capability through a stable adapter contract. It does not import training code or know how the model was trained.

## Branch retirement and knowledge promotion

Before deleting or replacing a long-lived branch:

1. compare it with its intended destination;
2. identify commits, contracts, protocols, or evidence that do not exist elsewhere;
3. promote stable knowledge into code, tests, model cards, research docs, or normative documents;
4. preserve useful dated experiment evidence as historical records;
5. confirm no stable contract depends on branch-only knowledge;
6. only then retire the branch.

A merged implementation is not enough if the reasoning boundary, acceptance contract, or provenance needed to maintain it exists only in an old PR, mixed development guide, or chat transcript.

## Agent execution pattern

For one bounded PR, use `.github/pull_request_template.md` as the compact task ledger.

For work that spans sessions/PRs, use the [execution-plan contract](exec-plans/README.en.md). Do not create repository files only to store transient chain-of-thought or session scratch.

The durable state that matters is: target capability, exact refs, invariant, evidence, decisions, ordered work, promotion state, residual gaps, and one resumable next action.

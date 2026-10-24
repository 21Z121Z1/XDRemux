---
name: pstack-engineering-principles
description: Apply pstack's boundary discipline, root-cause debugging, evidence-backed subtraction, behavior tests, and direct verification to XDRemux. Use when changing validation, media or helper boundaries, product ownership, a regression, or maintained complexity.
---

# XDRemux engineering principles

Read [AGENTS.md](../../../AGENTS.md) and the responsible implementation first. Apply a principle when it changes a concrete decision. Do not turn these principles into extra product policy or a checklist for every edit.

## Validate each boundary

Validate external values where responsibility or trust changes. Pass explicit typed facts to pure policy code. Keep intent and decisions in the Rust engine, effects and publication in runtime, and Apple framework operations in the adapter. Do not expose adapter wire types as engine policy.

Media bytes, parsed containers, files, persisted state, and helper input or output remain system boundaries. The same applies to native observations, decoded resources, and final publication bytes. An internal typed value does not remove these boundaries.

A type or earlier parser pass does not prove data completeness, native acceptance, or unchanged final bytes. Preserve required bounds, source relationships, capability checks, and final readback. Required validation failures must stop the operation.

Remove repeated validation only after proving that both checks inspect the same unchanged fact within the same trust boundary.

## Fix the first incorrect decision

Reproduce the symptom. Trace the first wrong decision or unhandled legitimate state from the actual error and runtime state. If the deciding fact is unclear, add bounded observations.

A guard can fix a legitimate state at the correct boundary. It hides the fault if an earlier producer should never have created that state.

Do not conceal failures with retries, clamps, swallowed errors, invented metadata, discarded resources, or empty successful results. If successive fixes share an assumption, test that assumption. Inspect other callers only where the same demonstrated mechanism can reach them. Preserve useful failure evidence while narrowing the fix.

## Remove unnecessary maintained behavior

Before removing an API or representation, trace its producers, consumers, dynamic uses, and public compatibility. Remove a maintained obligation only when its purpose is demonstrably gone. Prefer the smallest complete implementation to another mode, wrapper, cache, validator, or backend.

Preserve format, provenance, native ABI, publication, and recovery obligations. Code that looks defensive or lacks an obvious direct caller does not justify removal.

For each proposed removal, name what disappears and what remains. State any behavior given up and the strongest reason to retain it. Moving the same complexity does not simplify the system.

## Test observable behavior

Exercise the interface that its user calls. Assert concrete results, failure behavior, durable state, or external effects. A wrong output or missing effect must fail the test. A behavior-preserving internal rewrite should still pass.

Keep ordering and interaction assertions when they represent an external native lifecycle, protocol, atomic publication, or recovery contract. Keep cross-file assertions when repository structure is the contract. Distinguish those checks from assertions that repeat a helper call, variable name, or constant without executing its consumer.

## Verify the actual result

Inspect the actual artifact and observed value. Compilation, file timestamps, cached reports, and agent summaries do not establish functional behavior. Prefer an existing deterministic command to a new verification subsystem.

Match evidence to the claim. Portable structure does not establish native framework support. Native parsing does not establish device editing. Missing required source data, capability, or validation must fail before publication damages an input or existing output.

Follow the repository's exact-HEAD receipt rules. These rules do not automatically authorize commits, pushes, or publication. Do not add another ledger.

When a principle changes a decision, report that decision and its evidence. Cite only principles and artifacts actually read during the session.

Adapted from five pstack principle skills at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`. Original paths and hashes are in [sources.json](sources.json). See [LICENSE](LICENSE).

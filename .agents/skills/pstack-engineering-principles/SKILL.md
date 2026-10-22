---
name: pstack-engineering-principles
description: Apply pstack's boundary discipline, root-cause debugging, evidence-backed subtraction, behavior tests, and direct verification to XDRemux. Use when changing validation, media or helper boundaries, product ownership, a regression, or maintained complexity.
---

# XDRemux engineering principles

Read [AGENTS.md](../../../AGENTS.md) and the relevant owner first. Apply the principles that change a concrete decision. Do not turn them into additional product policy or a mandatory checklist for every edit.

## Boundary discipline

Validate external values where ownership or trust changes, then pass explicit typed facts to pure policy code. Keep product intent and decisions in the Rust engine, effects and publication in the Rust runtime, and Apple framework operations in the adapter. Do not export adapter wire types as engine policy.

Media bytes, parsed containers, files and persisted state, helper-process input/output, native framework observations, decoded resources, and final publication bytes remain system boundaries even when an internal function carries them in a typed value. A type or a successful earlier parser does not prove complete data, native acceptance, or unchanged final bytes. Preserve required bounds, source relationships, capability checks, final readback, and fail-closed behavior. Remove repeated validation only after proving it checks the same unchanged fact within the same trust boundary.

## Fix root causes

Reproduce the symptom. Trace it to the first wrong decision or unhandled legitimate state. Use the actual error and runtime state; add bounded observations when the deciding fact is unclear. A guard is a fix when that state is legitimate and belongs at that boundary. It hides the fault when an earlier producer should never have created the state.

Do not add retries, clamps, swallowed errors, invented metadata, discarded resources, or successful-empty defaults to conceal a failure. Test the assumption when successive fixes rely on it. Check other callers only where the same demonstrated mechanism can reach them. Preserve useful failure evidence while narrowing the fix.

## Subtract before adding

Trace actual producers, consumers, dynamic use, and public compatibility before removing an API or representation. Remove maintained obligations whose purpose is demonstrably gone. Prefer the smallest coherent implementation to another mode, wrapper, cache, validator, or backend.

Do not delete a documented format, source-provenance, native ABI, publication, or recoverability obligation because it looks defensive or lacks an obvious direct call. Name what disappears, what remains, the behavior given up, and the strongest reason to retain it. A change that only moves the same complexity has not simplified the system.

## Test observable behavior

Drive the interface its user actually calls. Assert concrete results, failure behavior, durable state, or external effects. Imagine a wrong output or omitted effect: the test must fail. Imagine a behavior-preserving internal rewrite: the test should survive.

Keep ordering or interaction assertions when they encode an external native lifecycle, protocol, atomic publication, or recoverability contract. Keep cross-file policy assertions when the structure itself is the repository contract. Do not confuse those with tests that restate a helper call, variable name, or a constant without executing its consumer.

## Prove the result directly

Read the real artifact and actual observed value. A compile, file timestamp, cached report, or agent self-report is not functional proof. Prefer an existing deterministic command over a new verification subsystem.

Match the evidence to the claim. Portable structure does not establish native framework support. Native parsing does not establish device editing. Missing required source data, capability, or validation must fail before publication damages an input or existing output. Follow the repository's exact-HEAD receipt rules without automatically committing, pushing, publishing, or adding another ledger.

Report the concrete decision and supporting evidence when a principle materially shaped the change. Cite only principles and artifacts actually read in the session.

Adapted from five pstack principle skills at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`. Original paths and hashes are in [sources.json](sources.json). See [LICENSE](LICENSE).

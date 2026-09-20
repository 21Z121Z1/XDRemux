# XDRemux Agent Guide

English | [简体中文](AGENTS.zh-CN.md)

XDRemux has one product stack: the Rust `xdremux` CLI, runtime, engine, and media crates. Swift is only the narrow Apple framework adapter. Python is research and training tooling.

Read [development](docs/development.en.md) for ownership, [testing policy](docs/quality/testing.en.md) for evidence selection, [validation](docs/validation/README.en.md) for exact-HEAD receipts, and [Apple features](docs/apple-features.en.md) for the platform boundary.

## Invariants

- Preserve real source facts and verified resource relationships. Do not invent Apple or vendor metadata only to satisfy a parser.
- `xdremux-engine` owns source facts, user intent, capabilities, deterministic policy, and the conversion plan. `xdremux-runtime` owns execution, validation order, filesystem effects, and atomic or recoverable publication.
- The Apple adapter performs only operations that require Apple frameworks. It returns facts or primitive results. It does not own product policy.
- Missing required source data, capability, or validation must fail closed. A failed operation must not damage an existing input or published output.
- Structural, native-framework, and device evidence are different. A parser pass does not prove Apple Photos editing or gallery behavior.
- Keep public fixture bytes and provenance exact. Keep private or device-only media outside public Git and public artifacts.

## Change process

1. Derive current branch, PR, changed-path, and CI state from Git and GitHub. Do not trust copied status in prose.
2. Read the canonical owner, nearby tests, and applicable contracts before editing.
3. Make the smallest coherent change. Do not create a second product policy in Swift, Python, scripts, or research code.
4. Add a regression that checks observable behavior when practical. Do not bind a test to a local variable name or helper call unless that source structure is the contract.
5. Run the smallest complete evidence set. Bind completion evidence to the committed `HEAD` when product code or validation infrastructure changes.
6. Report only the behavior that the evidence proves. State device-dependent gaps explicitly.

Write the English canonical document first. Finalize it before translating the same scope and limits into Chinese.

## Research lifecycle

Product implementation, durable research evidence, and diagnostic probes are different assets. Each experiment must end in one state:

- **PROMOTE**: move the verified contract into canonical code, tests, or current documentation, then remove the superseded probe.
- **ARCHIVE**: keep only the reproducible evidence, provenance, result, and residual gap that have long-term value.
- **DISCARD**: remove the probe when its conclusion is absorbed or it has no durable value.

A branch name, commit count, structural artifact, or offline metric does not promote a feature. Use an execution plan only for work that spans sessions or PRs, depends on private fixtures or devices, or has a long promotion ladder. Do not store chat logs, chain-of-thought, session journals, branch SHAs, ahead/behind counts, or current workflow status as stable repository knowledge.

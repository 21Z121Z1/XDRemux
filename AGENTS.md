# XDRemux agent guide

English | [简体中文](AGENTS.zh-CN.md)

Use this guide to change and verify XDRemux. The Rust `xdremux` CLI, runtime, engine, and media crates form the only product stack. Swift provides the Apple framework adapter. Python provides research and training tools and isolated model primitives under `model_adapters/`. Model primitives return observations. Camera selection, admission, and conversion policy remain in Rust.

Read the [development guide](docs/development.en.md) for component ownership and the [testing policy](docs/quality/testing.en.md) to select evidence. The [validation runbook](docs/validation/README.en.md) explains receipts for a specific commit. The [Apple feature guide](docs/apple-features.en.md) defines platform limits.

## Preserve the product contracts

- Preserve facts from the source media and verified relationships between resources. Do not invent Apple or vendor metadata to make a parser accept a file.
- Keep source facts, user intent, capabilities, deterministic policy, and conversion plans in `xdremux-engine`.
- Keep execution, validation order, filesystem effects, and atomic or recoverable output publication in `xdremux-runtime`.
- Limit the Apple adapter to operations that require Apple frameworks. Return framework observations or operation results. Keep product decisions in Rust.
- Stop an operation when required source data, capabilities, or validation are missing. Failure must not damage an input or an existing published output.
- Distinguish structural checks, native framework checks, and device tests. A parser pass does not prove Photos editing or gallery behavior.
- Preserve the bytes and provenance of public fixtures. Keep both private media and device-only media outside public Git and public artifacts.

## Make and verify a change

1. Read the current branch, PR, changed paths, and CI state from Git and GitHub. Do not use copied status text as current evidence.
2. Read the implementation that owns the behavior, its nearby tests, and the applicable contracts.
3. Make the smallest complete change. Do not create a second product policy in Swift, Python, scripts, or research code.
4. When practical, add a regression test for observable behavior. Test a local variable name or helper call only when that source structure is itself the contract.
5. Run the smallest complete set of checks. For product or verification-infrastructure changes, bind completion evidence to the committed `HEAD`.
6. Report the behavior that the evidence proves. State any remaining device-dependent gap.

Finalize the canonical English document before translating it into Chinese. Preserve the same technical scope and limits in both versions.

## Use the project skills

Use the [pstack workflow](.agents/skills/pstack-workflow/SKILL.md), [behavioral TDD](.agents/skills/pstack-tdd/SKILL.md), [engineering principles](.agents/skills/pstack-engineering-principles/SKILL.md), [review](.agents/skills/pstack-review/SKILL.md), and [prose cleanup](.agents/skills/pstack-unslop/SKILL.md) when their descriptions apply. These are project adaptations pinned to a source version. They do not require the Cursor runtime.

Use [XDRemux verification](.agents/skills/xdremux-verify/SKILL.md) for CLI procedures and evidence selection.

| Contract | Existing checks |
| --- | --- |
| HDR resources and transforms retain their relationships. | HEIF and codec graph tests, RGB16 regressions, and ImageIO readback in `ci.yml`. |
| Native execution and edit state are complete. | Texture receipt and state regressions, plus native consumer checks. |
| Failed or competing publication preserves media. | Runtime batch and Live Photo failure tests, plus app model tests. |
| Evidence identifies the tested source and host. | The completion gate binds receipts to `HEAD`. Native and device claims require the relevant environment. |
| CI has reusable owners. | Workflow tests assign product checks to `ci.yml` and research checks to `research.yml`. |

## Finish research work

Keep product implementation, durable research evidence, and diagnostic probes distinct. Finish each experiment in one of these states:

- **PROMOTE**. Move the verified contract into product code, tests, or current documentation. Remove the superseded probe.
- **ARCHIVE**. Retain the reproducible evidence, provenance, result, and remaining gap that have long-term value.
- **DISCARD**. Remove a probe when its conclusion is already incorporated or it has no durable value.

A branch name, commit count, structural artifact, or offline metric does not establish product support. Use an execution plan only for work that spans sessions or PRs, depends on private fixtures or devices, or has several stages before promotion.

Do not store chat logs, chain-of-thought, session journals, branch SHAs, ahead/behind counts, or current workflow status as stable repository knowledge.

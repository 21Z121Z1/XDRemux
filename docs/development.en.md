# Development and Builds

English | [简体中文](development.md)

XDRemux has one product core: the Rust workspace. The only public CLI is the Rust `xdremux` binary. New product behavior belongs in Rust. Swift is limited to the Apple framework adapter, and Python is research/training tooling rather than a second XDRemux runtime.

For user-facing command-line behavior, see the [CLI reference](cli.en.md).

## Canonical development loop

Use the Rust workspace for product changes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xdremux-cli -- --help
```

The canonical product stack is:

```text
xdremux-cli
    ↓
xdremux-runtime
    ↓
xdremux-engine
    ↓
source / classification
motion-photo / hdr / metadata
container / heif / codec / format
    ↓
portable providers + platform adapters
```

Keep product intent at the top of this stack. Do not expose codec, camera-tail, source-generation, routing, or container implementation choices as user options unless they represent a distinct user outcome.

## Rust workspace ownership

| Crate | Responsibility |
| --- | --- |
| `xdremux-cli` | The only public user CLI and command contract. |
| `xdremux-runtime` | Filesystem execution, publication, batch reliability, recovery, and platform capability coordination. |
| `xdremux-engine` | Product intent, conversion planning, capability requirements, platform-independent facts, and orchestration. |
| `xdremux-source` / `xdremux-classification` | Source probing, asset identity, and classification. |
| `xdremux-motion-photo` | Motion Photo parsing and Live Photo media semantics. |
| `xdremux-hdr` / `xdremux-metadata` | HDR/Gain Map math and metadata primitives. |
| `xdremux-container` / `xdremux-heif` / `xdremux-codec` / `xdremux-format` | Container, HEIF, codec, JPEG/EXIF/TIFF/ISOBMFF primitives. |

A lower crate may provide a format primitive without making it a product mode. Runtime and engine own the decision to use that primitive.

## Apple platform capabilities

`Sources/XDRemuxAppleAdapter/` is the only Swift package source. It contains the Apple framework primitives that the Rust runtime cannot call portably. The former Swift conversion/core/oracle targets have been removed; new product behavior must not enter Swift.

The boundary is intentionally narrow:

- Rust owns requests, results, policy, routing, naming, classification, HDR behavior, Motion Photo behavior, batch behavior, validation policy, and fallback decisions.
- Apple code calls Apple frameworks and returns observations or operation results defined by Rust semantics.
- Do not add new cross-platform product policy to Swift.
- Do not return business conclusions such as “convertible” or “valid portrait” when the adapter can return lower-level framework facts and Rust can decide the policy.

`xdremux-apple-adapter` is a distributable platform component, not a user CLI. The current CLI/runtime boundary is a versioned JSON helper protocol with bounded process lifetime and separate machine-readable stdout/diagnostic stderr. `xdremux-runtime` owns that transport; `xdremux-engine` does not know about processes, paths, JSON, Swift, or XPC. The macOS app also invokes the Rust `xdremux` binary and does not link Swift product code.

For a sandboxed macOS app, prefer XPC when the Apple capability process needs separate sandboxing, entitlements, lifecycle, or crash isolation. The transport must remain replaceable without changing engine or public CLI semantics. Use an in-process C ABI only for a capability that actually benefits from direct in-process interoperability; do not make FFI the default architecture merely to avoid a helper process.

ImageIO auxiliary-resource probing is the first migrated operation. The Swift adapter reports only framework observations such as Gain Map, disparity, Portrait Effects Matte, and semantic mattes. Rust owns the rule that decides whether those facts satisfy the Portrait editing contract.

SwiftPM contains only the adapter executable. Its build verifies the platform primitive boundary; Rust tests and Rust-driven consumer checks define the product contract.

## Python tooling

The Python package requires Python 3.11 or newer and contains only research/training code. It does not install a CLI, participate in the runtime, or define product semantics.

Install the tooling only when a Python research workflow requires it:

```bash
python -m pip install -e .
python -m unittest Tests.test_apple_reverse_key1_training
```

Research dependencies currently include `Pillow` and `numpy`. The optional `training` dependency adds PyTorch.

Training and evaluation scripts may remain in the Python research package because they are tooling, not a second XDRemux implementation.

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/` | Canonical Rust product stack. |
| `Sources/XDRemuxAppleAdapter/` | Versioned Apple platform process adapter consumed by the Rust runtime. |
| `xdremux_py/` | Research/training tooling; no product CLI or converter. |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI presentation shell that invokes the Rust CLI. |
| `Tests/` | Rust acceptance policy tests and validation harnesses. |
| `fixtures/` | Versioned real media fixtures used by strict gates. |
| `scripts/` | Build, evaluation, migration, and acceptance utilities. |
| `docs/` | Current guidance and historical research records. |
| `Models/` | Optional research models and model documentation. |

New user-visible behavior belongs in Rust. Swift changes must remain framework primitives, and Python changes must remain research tooling.

## Apple helper lifecycle

A helper invocation must be bounded, must keep machine-readable stdout separate from diagnostics, and must explicitly reap the child process. Do not let transport concerns leak into engine models.

Private Apple API compatibility must be checked at runtime. Do not call a private Objective-C selector with an assumed ABI when the runtime method signature does not match a supported form.

## macOS app

The app is in `apps/macos/XDRemuxApp/`. It invokes the Rust CLI for product work and keeps only presentation state, queue management, and receipt translation in Swift.

Common commands:

```bash
scripts/build_and_run.sh run
scripts/build_and_run.sh build
scripts/build_and_run.sh debug
scripts/build_and_run.sh verify
scripts/build_and_run.sh logs
scripts/build_and_run.sh clean
```

The app bundles the Rust CLI and Apple adapter as helpers. It owns presentation state and receipt translation only; conversion policy remains in Rust.

## Debug and research controls

Environment variables exist for encoding diagnostics, scratch retention, style rendering, and research model selection.

Do not add a research environment variable to a normal user command unless the product behavior requires it. Do not document a research switch as a stable public interface unless the canonical Rust product path and tests depend on it.

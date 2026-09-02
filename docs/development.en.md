# Development and Builds

English | [简体中文](development.md)

XDRemux has one product core: the Rust workspace. New product behavior belongs in Rust. Swift is a migration-time Apple capability layer, and Python is migration/research tooling rather than a second XDRemux runtime.

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
| `xdremux-cli` | The only public CLI and command contract. |
| `xdremux-runtime` | Filesystem execution, publication, batch reliability, recovery, and platform capability coordination. |
| `xdremux-engine` | Product intent, conversion planning, capability requirements, and orchestration. |
| `xdremux-source` / `xdremux-classification` | Source probing, asset identity, and classification. |
| `xdremux-motion-photo` | Motion Photo parsing and Live Photo media semantics. |
| `xdremux-hdr` / `xdremux-metadata` | HDR/Gain Map math and metadata primitives. |
| `xdremux-container` / `xdremux-heif` / `xdremux-codec` / `xdremux-format` | Container, HEIF, codec, JPEG/EXIF/TIFF/ISOBMFF primitives. |

A lower crate may provide a format primitive without making it a product mode. Runtime and engine own the decision to use that primitive.

## Apple platform capabilities

`Sources/XDRemuxAppleFeatures/` remains while Photographic Styles, Portrait, RAW, Vision, Core ML, AVFoundation, and other Apple-framework capabilities are migrated behind the Rust capability model.

The target architecture is a narrow platform adapter:

- Rust owns requests, results, policy, routing, naming, classification, HDR behavior, Motion Photo behavior, batch behavior, and fallback decisions.
- The Apple layer calls Apple frameworks and returns capability results.
- Do not add new cross-platform product policy to Swift.

Use a stable C ABI for the Rust/Swift boundary where direct in-process interoperability is required. Keep FFI-specific unsafe code isolated from the safe engine/runtime crates.

`CoreImageRAWDiagnostics` remains a developer-only Swift target:

```bash
swift build --target CoreImageRAWDiagnostics
```

Swift tests are still useful for Apple capability acceptance and migration oracles, but they are not the canonical CLI contract.

## Python tooling

The Python package requires Python 3.11 or newer and is retained for migration-time conformance, real-fixture oracles, and research/training workflows. It does not install a CLI and must not define new product semantics.

Install the tooling only when a Python oracle or research workflow requires it:

```bash
python -m pip install -e .
python -m unittest discover -s Tests -v
```

Runtime dependencies currently include `pillow-heif`, `Pillow`, `numpy`, and `piexif`. The optional `training` dependency adds PyTorch.

Training and evaluation scripts may remain in Python because they are research tooling, not a second XDRemux implementation.

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/` | Canonical Rust product stack. |
| `Sources/XDRemuxAppleFeatures/` | Apple capability implementation and migration-time validation. |
| `Sources/XDRemuxCore/` | Legacy Swift core retained only while replacement evidence is incomplete. |
| `Sources/XDRemuxCLI/` | Legacy Swift CLI retained only while Apple migration work still references it. |
| `Sources/CoreImageRAWDiagnostics/` | Developer RAW diagnostics. |
| `xdremux_py/` | Migration oracles and research/training tooling; no product CLI. |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI app during product-stack migration. |
| `Tests/` | Rust/Swift/Python acceptance tests and validation harnesses. |
| `fixtures/` | Versioned real media fixtures used by strict gates. |
| `scripts/` | Build, evaluation, migration, and acceptance utilities. |
| `docs/` | Current guidance and historical research records. |
| `Models/` | Optional research models and model documentation. |

Legacy Swift/Python code should receive only migration, conformance, safety, or deletion work. New user-visible behavior belongs in Rust.

## Apple helper processes

Some Apple-feature operations compile or run helper programs from package resources.

The helper toolchain hashes source content and caches compatible built tools. Keep helper invocations bounded and keep machine-readable stdout separate from diagnostics when the protocol requires it.

Private Apple API compatibility must be checked at runtime. Do not call a private Objective-C selector with an assumed ABI when the runtime method signature does not match a supported form.

## macOS app

The app is in `apps/macos/XDRemuxApp/`.

Common commands:

```bash
scripts/build_and_run.sh run
scripts/build_and_run.sh build
scripts/build_and_run.sh debug
scripts/build_and_run.sh verify
scripts/build_and_run.sh logs
scripts/build_and_run.sh clean
```

During migration, the app may still link Swift package code. Do not treat that dependency as ownership of the canonical conversion policy; move reusable product behavior into Rust first.

## Debug and research controls

Environment variables exist for encoding diagnostics, scratch retention, style rendering, and research model selection.

Do not add a research environment variable to a normal user command unless the product behavior requires it. Do not document a research switch as a stable public interface unless the canonical Rust product path and tests depend on it.

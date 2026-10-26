# Develop and build XDRemux

English | [简体中文](development.md)

XDRemux has one product core: the Rust workspace. The only public CLI is the Rust `xdremux` binary. Rust owns new product behavior. Swift and its native targets provide Apple framework operations. Python provides research and training tools.

This guide describes component ownership, build commands, and packaging limits. User commands are defined in the [CLI reference](cli.en.md).

## Run the product checks

Run these commands from the repository root. The portable conversion stack requires a Rust toolchain and libheif with HEVC support. Native Apple checks also require the supported macOS and Xcode environment.

```bash
cargo metadata --locked --no-deps --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- --deny warnings
cargo test --locked --workspace --all-targets
cargo run --locked -p xdremux-cli -- --help
```

The CLI passes work to the runtime. The runtime executes the engine's plan through the source, media, container, and codec crates. Portable providers and Apple adapters supply the required capabilities.

Keep user intent in the CLI, runtime, and engine. Expose an encoding, source-generation, routing, camera-tail, or container choice only when it represents a distinct user outcome.

## Keep each responsibility in its owner

| Crate | Responsibility |
| --- | --- |
| `xdremux-cli` | Public commands and their user-visible behavior. |
| `xdremux-runtime` | Filesystem operations, output publication, batch execution, recovery, and platform capability coordination. |
| `xdremux-engine` | User intent, source facts, deterministic policy, capability requirements, and conversion plans. |
| `xdremux-source` and `xdremux-classification` | Source inspection, asset identity, and classification. |
| `xdremux-motion-photo` | The LibLivePhoto interface used by the product, image geometry, and pair publication. |
| `xdremux-hdr` and `xdremux-metadata` | HDR and Gain Map calculations and metadata operations. |
| `xdremux-container`, `xdremux-heif`, `xdremux-codec`, and `xdremux-format` | Product container and codec operations. Shared binary types come from `liblivephoto-format`. |

A format operation in a lower crate does not automatically become a product mode. The engine and runtime decide when to use it. See [LibLivePhoto integration](liblivephoto-integration.en.md) for that library's boundary and the [architecture guide](architecture.en.md) for the product flow.

## Use Apple frameworks through the adapter

The Swift package publishes one executable, `xdremux-apple-adapter`. Its Swift source is in `Sources/XDRemuxAppleAdapter/`. The package also contains the `XDCinematicNative` and `XDTextureNative` targets, which provide native Apple operations. The former Swift conversion, core, and oracle products have been removed.

Keep the boundary narrow:

- Rust owns requests, results, policy, routing, naming, classification, HDR, Motion Photo, batch behavior, validation, and fallback decisions.
- The adapter calls Apple frameworks and returns observations or operation results defined by the Rust contract.
- Do not add cross-platform product policy to Swift or the native targets.
- If framework facts are sufficient, return those facts. Let Rust decide whether an input is convertible or has valid Portrait resources.

The adapter ships as a platform component, not another public CLI. The runtime communicates with it through a versioned JSON protocol. Each helper process has a bounded lifetime. Machine-readable output uses stdout, and diagnostics use stderr.

`xdremux-runtime` owns this transport. The engine does not launch helper processes or implement the transport. The macOS app also invokes the Rust CLI for product work.

The adapter's ImageIO probe reports auxiliary resources such as Gain Map, disparity, Portrait Effects Matte, and semantic mattes. Rust decides whether those observations satisfy the Portrait editing contract. An adapter build checks the platform implementation. Rust tests and Rust-driven consumer checks establish the product behavior.

### Choose a transport for a new capability

For a sandboxed app, prefer XPC when a capability process needs separate sandboxing, entitlements, lifecycle management, or crash isolation. This guidance applies to future transport changes. The current adapter uses the JSON process protocol.

A transport change must preserve engine and public CLI behavior. Use an in-process C ABI only when the capability benefits from in-process interoperability. Avoid choosing FFI solely to remove a helper process.

Every helper invocation must have a time limit and must explicitly reap its child process. Keep diagnostics separate from machine-readable output. Check private Apple API compatibility at runtime. Do not call a private Objective-C selector through an ABI that its runtime signature does not support.

## Build and verify the macOS package

The complete macOS product targets Apple silicon (`arm64`) and macOS 27 or newer. The app, Swift package, and product archive use a macOS 27.0 deployment target. The adapter rejects unsupported hosts before it reports capabilities.

The [`rust-cli-core.yml`](../.github/workflows/rust-cli-core.yml) workflow builds one arm64 product package on the `xcode-27` runner. It uses [`scripts/distribution/`](../scripts/distribution/) and retains the root Swift package and both native targets. Separate Rust core jobs run on Linux, Windows, and macOS. These jobs do not establish the complete Apple feature set on each platform.

The package includes:

- The Rust CLI and the Apple adapter.
- Checksum-pinned native dependencies, their source archives, and their license notices.
- The required dynamic libraries, with loader-relative paths and ad-hoc signatures.

Before upload, CI extracts the archive outside the checkout. It launches the packaged programs with `env -i` and a `PATH` that contains only system directories. The check converts a real Samsung HEIF Motion Photo and an OPPO ProXDR fixture. It validates the Live Photo pair and the HDR output, checks input preservation, and rejects a second write to the existing pair.

The dyld log must identify the bundled libheif. It must not show a non-system library loaded from outside the package. Run the portable packaging tests with:

```bash
python3 -m unittest tests.test_macos_distribution
```

The package check establishes portable conversion on the tested arm64/macOS 27 host. Photographic Styles, Texture Styles, and Cinematic require their own framework, runtime, hardware, and native acceptance evidence.

No Intel product package is built. An API introduced before macOS 27 does not lower the complete product or private-SPI support requirement. Archive timestamps and owners are normalized for fixed staged bytes. This does not establish compiler-level reproducibility. The packages are CI artifacts with ad-hoc signatures, not notarized releases.

## Install Python research tools

The Python package requires Python 3.11 or newer. It does not install a CLI, participate in the product runtime, or define product behavior.

Install it only for a research workflow that needs Python:

```bash
python -m pip install -e .
python -m unittest tests.test_apple_reverse_key1_training
```

The package depends on `Pillow` and `numpy`. The optional `training` dependency adds PyTorch. Training and evaluation code can remain in Python because it is research tooling.

The separate `model_adapters/geocalib/` component is a runtime ML primitive, outside the research package. Rust invokes the installed `xdremux-geocalib` helper through a bounded process boundary. It returns observations only; the engine owns their interpretation and admission. See [installation and the observation contract](../model_adapters/geocalib/README.en.md). Product archives do not include its Python environment, weights, FFmpeg or ExifTool. Automatic Cinematic input requires those local dependencies; explicit focal input bypasses them. No conversion downloads models automatically.


## Find the relevant files

| Path | Purpose |
| --- | --- |
| `crates/` | Rust product implementation and Rust tests. |
| `Sources/XDRemuxAppleAdapter/` | Swift Apple framework adapter called by the runtime. |
| `Sources/XDCinematicNative/` and `Sources/XDTextureNative/` | Native operations used by the adapter. |
| `model_adapters/` | Local model primitives that return observations to Rust. |
| `xdremux_py/` | Python research and training tools. |
| `apps/macos/XDRemuxApp/` | SwiftUI app, Xcode project, app tests, and app build scripts. |
| `tests/` | Python regressions, repository policy tests, verification self-tests, and small synthetic fixtures. |
| `fixtures/` | Versioned real media used by strict validation checks. |
| `scripts/` | [Repository automation](../scripts/README.en.md) for CI, validation, Apple checks, diagnostics, distribution, and performance. |
| `docs/` | Current guidance and validation records. Historical audits are in `docs/history/`. |
| `research/` | OPPO Styles, Texture Styles, video Styles, and Palette investigations. |
| `benchmarks/` | Versioned performance baselines. Generated measurements remain separate. |
| `research/oppo_styles/models/` | Optional research models and their documentation. |

## Build and inspect the macOS app

The app invokes the Rust CLI. Swift owns presentation state, the queue, and conversion of CLI receipts into app state. The app bundles the CLI and Apple adapter as helpers. Conversion policy remains in Rust.

Before `clean`, check `XDREMUX_DERIVED_DATA` if it is set. The script deletes that directory. The default is `.build/xcode`.

Run the app script from the repository root. Select one operation:

| Command | Result |
| --- | --- |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh run` | Build Release and open the app. |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh build` | Build Release without opening the app. |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh debug` | Build Debug and open the app. |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh verify` | Build the app and run the checks listed in the script. |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh logs` | Print the last 200 lines of the saved build log. |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh clean` | Delete the configured derived-data directory. |

## Keep research controls separate

Environment variables support encoding diagnostics, retained scratch files, style rendering, and research model selection.

Add a research variable to a normal user command only when product behavior requires it. Treat a research switch as a stable public interface only when the Rust product implementation and tests depend on it.

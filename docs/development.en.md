# Development and Builds

English | [简体中文](development.md)

Use this document when you maintain the released v1.4 Swift/Python line, integrate its Swift package, or build the macOS app.

New product development after v1.4 is moving to the Rust rewrite. Read the [system architecture](architecture.en.md) before cross-module work and the [transition roadmap](roadmap.en.md) before migration work. Do not infer the future architecture from the v1.4 directory layout below.

For command-line use, see the [CLI reference](cli.en.md).

## Release and development lines

`v1.4` is the final release that ships both the Swift and Python implementations.

Use the v1.4 release and the current `main` maintenance line for released Swift/Python behavior. Use the Rust rewrite branch for active migration implementation after you compare it with its intended base.

The programming language is not the architectural boundary. Stable ownership is defined by the capability and layer model in `architecture.en.md`.

## Toolchain

The v1.4 Swift package manifest sets:

- Swift tools version 6.0;
- minimum platform macOS 15;
- package default localization `en`.

The package currently uses Swift language mode 5 for its targets.

Build and test the Swift/Python line with:

```bash
swift build
swift test
python3 -m unittest discover -s Tests -v
```

Rust migration commands and crate-specific gates live on the active Rust branch. Do not add Rust commands to this v1.4 guide only to mirror branch-local implementation state; promote stable migration contracts into the architecture or roadmap instead.

## Swift package products

| Product | Type | Purpose |
| --- | --- | --- |
| `XDRemuxCore` | library | HDR conversion, HEIF/ISO-BMFF work, metadata, Motion Photo parsing, classification, and shared validation. |
| `XDRemuxAppleFeatures` | library | Apple Live Photo, Photographic Styles, Apple Portrait, and Apple-specific analysis. |
| `xdremux` | executable | Swift command-line interface. |

`CoreImageRAWDiagnostics` is a developer-only executable target. It is not a public package product.

Build it with:

```bash
swift build --target CoreImageRAWDiagnostics
```

## Package integration

Add the released line as a package dependency by using a release version or an exact revision appropriate for your integration. A dependency on `main` follows the maintenance branch and can receive API changes.

The repository's v1.4 source remains compatible with the package products described below.

Use `XDRemuxCore` when you need the standard conversion pipeline.

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
let request = ConversionRequest(
    input: input,
    output: OutputTarget.file(outputURL).destination(for: input),
    configuration: ConversionConfiguration()
)

let result = try ConversionEngine.convert(request)
```

Use `XDRemuxAppleFeatures` for Apple-specific conversion engines.

## Repository layout

The following layout describes the v1.4 Swift/Python implementation. It is an implementation map, not the system architecture.

| Path | Purpose |
| --- | --- |
| `Sources/XDRemuxCore/` | Core conversion and format logic. |
| `Sources/XDRemuxAppleFeatures/` | Apple-specific conversion and validation. |
| `Sources/XDRemuxCLI/` | Swift CLI parser and command routing. |
| `Sources/CoreImageRAWDiagnostics/` | Developer RAW diagnostic target. |
| `xdremux_py/` | Cross-platform Python implementation. |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI app. |
| `Tests/` | Swift tests, Python policy tests, and validation harnesses. |
| `fixtures/` | Versioned real Motion Photo fixtures used by strict CI gates. |
| `scripts/` | Build, evaluation, and acceptance utilities. |
| `docs/` | Current documentation and historical validation records. |
| `Models/` | Optional research models and model documentation. |

When code moves to Rust, map behavior by capability contract and evidence. Do not reproduce these directories one-for-one as crates.

## Apple helper processes

Some Apple-feature operations compile or run helper programs from package resources.

The helper toolchain hashes source content and caches compatible built tools. The implementation uses bounded helper invocations and separates machine-readable stdout from diagnostics where the helper protocol requires it.

Private Apple API compatibility must be checked at runtime. Do not call a private Objective-C selector with an assumed ABI when the runtime method signature does not match a supported form.

The macOS 27 compatibility path in the style-response helper checks known initializer and style-apply ABI shapes before calling them.

These helpers are v1.4 execution mechanisms. In the Rust architecture, Apple-only behavior belongs behind explicit operation-scoped adapter capabilities rather than inside the pure semantic engine.

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

The app links the Swift package. It does not use the CLI as a subprocess for core conversion.

If the app remains SwiftUI after the Rust transition, integrate Rust through a narrow library or FFI composition boundary. Do not move media policy into the UI layer.

## Python package

Python v1.4 requires Python 3.11 or newer.

Runtime dependencies include:

- `pillow-heif`;
- `Pillow`;
- `numpy`;
- `piexif`.

The optional `training` dependency adds PyTorch.

The installed console command is `xdremux-py`. The repository-local entry point is `python3 -m xdremux_py`.

The Python implementation remains a released v1.4 reference and a useful migration oracle where its behavior is independently supported by the product contract or evidence. Do not make Python a permanent Rust runtime dependency only to preserve migration parity.

## Debug and research controls

Environment variables exist for encoding diagnostics, scratch retention, style rendering, and research model selection.

Do not add a research environment variable to a normal user command unless the product behavior requires it.

Do not document a research switch as a stable public interface unless tests and the current product path depend on it.

The optional Reverse Key 1 model has a separate [model card](../Models/ReverseKey1Ensemble.model-card.en.md).

Research model output is a candidate, not product policy. Promotion into a future Rust product capability must satisfy the research gates in the [transition roadmap](roadmap.en.md).

## Completion gate

Repository agents must validate the exact committed `HEAD` before they claim completion.

The operating contract is in [AGENTS.md](../AGENTS.md). The acceptance runbook is in [validation/README.en.md](validation/README.en.md).

Use targeted evidence. A documentation-only change does not need the full real-photo matrix. A conversion-core change needs functional evidence in addition to static checks.

The completion receipt is bound to the commit, base commit, changed paths, and clean worktree. A later tracked edit invalidates the receipt.

Cross-module architecture or migration work must also identify the affected capability identifiers, owning layers, oracle/evidence, and residual gaps.

## Documentation changes

Current technical documentation follows the [technical writing guide](style-guide.en.md).

When a code change alters a documented command, output rule, format contract, architecture boundary, or acceptance rule, update the English document first and then update its Chinese translation.

Do not place a stable system rule only in a long-lived branch, PR description, or chat transcript. Promote it into the current architecture, roadmap, model card, test contract, or another appropriate normative document.

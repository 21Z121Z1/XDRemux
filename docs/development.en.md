# Development and Builds

English | [简体中文](development.md)

Use this document when you change XDRemux, integrate its Swift package, or build the macOS app.

For command-line use, see the [CLI reference](cli.en.md).

## Toolchain

The package manifest sets:

- Swift tools version 6.0;
- minimum platform macOS 15;
- package default localization `en`.

The package currently uses Swift language mode 5 for its targets.

Build and test:

```bash
swift build
swift test
python3 -m unittest discover -s Tests -v
```

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

Add the repository as a package dependency:

```swift
dependencies: [
    .package(
        url: "https://github.com/21Z121Z1/XDRemux.git",
        branch: "main"
    )
]
```

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

There is no stable release tag contract in the current package documentation. A dependency on `main` can receive API changes.

## Repository layout

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

## Apple helper processes

Some Apple-feature operations compile or run helper programs from package resources.

The helper toolchain hashes source content and caches compatible built tools. The implementation uses bounded helper invocations and separates machine-readable stdout from diagnostics where the helper protocol requires it.

Private Apple API compatibility must be checked at runtime. Do not call a private Objective-C selector with an assumed ABI when the runtime method signature does not match a supported form.

The macOS 27 compatibility path in the style-response helper checks known initializer and style-apply ABI shapes before calling them.

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

## Python package

The Python package requires Python 3.11 or newer.

Runtime dependencies include:

- `pillow-heif`;
- `Pillow`;
- `numpy`;
- `piexif`.

The optional `training` dependency adds PyTorch.

The installed console command is `xdremux-py`. The repository-local entry point is `python3 -m xdremux_py`.

## Debug and research controls

Environment variables exist for encoding diagnostics, scratch retention, style rendering, and research model selection.

Do not add a research environment variable to a normal user command unless the product behavior requires it.

Do not document a research switch as a stable public interface unless tests and the current product path depend on it.

The optional Reverse Key 1 model has a separate [model card](../Models/ReverseKey1Ensemble.model-card.en.md).

## Completion gate

Repository agents must validate the exact committed `HEAD` before they claim completion.

The acceptance runbook is in [validation/README.en.md](validation/README.en.md).

Use targeted evidence. A documentation-only change does not need the full real-photo matrix. A conversion-core change needs functional evidence in addition to static checks.

The completion receipt is bound to the commit, base commit, changed paths, and clean worktree. A later tracked edit invalidates the receipt.

## Documentation changes

Current technical documentation follows the [technical writing guide](style-guide.en.md).

When a code change alters a documented command, output rule, format contract, or acceptance boundary, update the English document first and then update its Chinese translation.

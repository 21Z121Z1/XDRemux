# XDRemux Development and Builds

English | [简体中文](development.md)

This document is for developers building the App, integrating the Swift Package, running validators, or changing the converter. See the [CLI reference](cli.en.md) for normal conversion use.

## Development environment

- macOS 15 or later.
- A Swift 6 toolchain.
- Use a current Xcode when building the macOS App, which adopts the latest SwiftUI APIs.
- Apple Portrait development requires `zstd`; JPEG portrait bridging requires `ultrahdr_app`.

## Swift Package products

| Product | Type | Purpose |
| --- | --- | --- |
| `XDRemuxCore` | Library | Conversion models, HDR, HEIF, metadata, batch behavior, and output validation |
| `XDRemuxAppleFeatures` | Library | Apple semantic analysis, Photographic Styles, and Portrait |
| `xdremux` | Executable | Public user CLI |
| `xdremux-dev` | Executable | Experimental controls, validators, and diagnostics |
| `XDRemuxSemanticHelper` | Executable | Isolated Apple semantic analysis |
| `XDRemuxHEVCEncoderHelper` | Executable | Isolated VideoToolbox HEVC encoding |
| `XDRemuxStyleValidationHelper` | Executable | Isolated Apple style-property validation |

Basic commands:

```bash
swift build
swift test
swift run xdremux --help
swift run xdremux-dev --help
```

Do not build only the CLI product when using Apple features. A complete `swift build` also produces the required helpers.

## Swift Package integration

```swift
dependencies: [
    .package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
]
```

Depend on `XDRemuxCore` or `XDRemuxAppleFeatures` as needed. The basic conversion entry point is:

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
var configuration = ConversionConfiguration()
configuration.eventHandler = { event in
    // Map structured events to the caller's log or UI.
}

let cancellation = ConversionCancellation()
configuration.cancellation = cancellation

let request = ConversionRequest(
    input: input,
    output: OutputTarget.file(outputURL).destination(for: input),
    configuration: configuration
)
let result = try ConversionEngine.convert(request)
```

Configure Apple features through `configuration.appleFeatureOptions` and call `AppleFeatureConversionEngine.convert(_:)`.

`XDRemuxCore` does not own terminal output, ANSI, localization, SwiftUI, or GitHub Actions behavior. Callers receive phase, warning, completed, and failed states through `ConversionEvent`, and cancel work through `ConversionCancellation`.

Until a stable tag is published, external projects following `main` must accept API changes. Switch to a semantic-version range after tagged releases are available.

## Prebuilt helpers

Apple-private or process-isolated work uses formal executable targets. Helpers are built ahead of time and placed in `Contents/Helpers` by the App. Runtime code does not search for source, calculate source hashes, or invoke `xcrun`, `swiftc`, or `clang`.

Current protocol identifiers are:

- `xdremux-semantic-helper-v1`
- `xdremux-hevc-encoder-helper-v1`
- `xdremux-apple-semantic-style-properties-probe-v1`

Helper stdout contains only versioned machine protocol data; stderr contains diagnostics. The App and CLI share one locator and support timeout and cancellation.

## Developer CLI

Internal controls are available only in `xdremux-dev`:

```bash
swift run xdremux-dev convert \
  --input IMG_001.heic \
  --family x7 \
  --input-processing hybrid \
  --oppo-compat auto \
  --oppo-camera-tail preserve \
  --tmap-format imageio \
  --diagnostics-dir diagnostics/
```

Retained internal options include `--family`, `--input-processing`, `--oppo-compat`, `--oppo-camera-tail`, `--tmap-format`, and `--diagnostics-dir`. The public `xdremux` command rejects them.

Validation commands:

```bash
swift run xdremux-dev validate-apple --input output.heic
swift run xdremux-dev validate-portrait --input output.heic --json validation.json
swift run xdremux-dev portrait-self-test
```

## macOS App

The App lives in `apps/macos/XDRemuxApp/`. It links the shared Swift Package directly and does not use the full CLI as a conversion service.

```bash
scripts/build_and_run.sh build
scripts/build_and_run.sh run
scripts/build_and_run.sh verify
scripts/build_and_run.sh debug
scripts/build_and_run.sh logs
scripts/build_and_run.sh logs --all
scripts/build_and_run.sh clean
```

`build` only builds. `run` builds and launches. `verify` also checks the bundle, helper signatures, and process. `debug` launches LLDB. `logs` filters to the `com.proxdr.XDRemuxApp` subsystem; `logs --all` shows the complete process log.

The default path uses quiet `xcodebuild`. Complete `build.log` and `.xcresult` diagnostics remain in XDRemux's DerivedData. `--verbose` streams the full build output.

## Repository layout

| Path | Purpose |
| --- | --- |
| `Package.swift` | SwiftPM product and target definitions |
| `Sources/XDRemuxCore/` | Platform-neutral conversion core |
| `Sources/XDRemuxAppleFeatures/` | Apple-specific features |
| `Sources/XDRemuxCLI/` | CLI parsing, localization, and output |
| `Sources/XDRemuxExecutable/` | Public CLI entry point |
| `Sources/XDRemuxDevExecutable/` | Developer CLI entry point |
| `Sources/XDRemux*Helper/` | Build-time process-isolated helpers |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI App |
| `Tests/` | Swift, Python, and validation tests |
| `scripts/` | Build, validation, and projection tools |
| `docs/` | User, engineering, validation, and research documentation |

## Acceptance rules

Every completion claim requires a completion-gate receipt bound to the final commit, but the gate must select evidence appropriate to the change.

New plans should declare `change_impact` (`documentation`, `non_output`, `output`, or `release`) and an `impact_rationale` explaining why generated files can or cannot change.

- Documentation changes: links, command examples, document structure, and public projection checks.
- CLI parsing changes: matching argument and output regressions.
- Conversion-core changes: unit tests plus real functional or integration evidence.
- App or helper changes: build, signature, runtime, or device evidence.
- Release or cross-module changes: the full matrix.

The existence of the completion gate does not justify running the real-photo matrix for every README edit. Finish and commit one coherent batch, then run one targeted plan for the final `HEAD`:

```bash
python3 scripts/agent_completion_gate.py run \
  --base <verified-base> \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

See the [validation guide](validation/README.md) for the plan schema and evidence requirements.

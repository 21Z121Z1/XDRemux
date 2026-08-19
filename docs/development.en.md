# XDRemux Development and Builds

English | [简体中文](development.md)

For people changing the converter, integrating the Swift package, or building the macOS app. For ordinary conversion use, see the [CLI reference](cli.en.md).

## Environment

- macOS 15 or newer
- Swift 6 toolchain
- Xcode, to build the macOS app
- `zstd` for the Apple portrait feature (`brew install zstd`)

## Swift package products

| Product | Kind | Purpose |
| --- | --- | --- |
| `XDRemuxCore` | Library | The conversion core: HDR, HEIF, metadata, batch, output validation |
| `XDRemuxAppleFeatures` | Library | Apple semantic analysis, Photographic Styles, portrait |
| `xdremux` | Executable | The command-line tool |

```bash
swift build
swift test
swift run xdremux --help
```

The CoreImage RAW probe remains available as a developer target, but is no longer vended as a public Swift package product. Build it explicitly when needed:

```bash
swift build --target CoreImageRAWDiagnostics
```

Its entry source is `Sources/CoreImageRAWDiagnostics/main.swift`; its argument contract is `DNG_DIRECTORY OUTPUT_DIRECTORY [MAX_SIZE]`.

## Integrating into your own project

```swift
dependencies: [
    .package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
]
```

Depend on `XDRemuxCore` or `XDRemuxAppleFeatures` as needed:

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
var configuration = ConversionConfiguration()
configuration.eventHandler = { event in
    // Route structured events into your own logging or UI.
}

let result = try ConversionEngine.convert(
    ConversionRequest(
        input: input,
        output: OutputTarget.file(outputURL).destination(for: input),
        configuration: configuration
    )
)
```

Apple features are configured through `configuration.appleFeatureOptions` and run through `AppleFeatureConversionEngine`.

`XDRemuxCore` stays out of terminals, ANSI, localization, SwiftUI, and CI output — callers receive stages, warnings, and results as `ConversionEvent` values.

There is no stable tag yet, so tracking `main` means accepting API changes.

## Helpers compiled at runtime

The Vision analysis, HEVC encoding, and style-property probing inside the Apple features run in separate processes. Their sources ship as package resources and are **compiled on first use**: `AppleNativeToolchain` hashes the source, calls `/usr/bin/xcrun` to build into the user cache directory, and reuses that build for identical sources afterwards.

Two consequences:

- The first run of an Apple feature pays a one-time compile; later runs do not.
- The cache directory is shared machine-wide, so a build is published through a temporary file and an atomic rename. Two XDRemux processes starting at once cannot read a half-written binary.

Every helper invocation is bounded by a timeout. Helper stdout carries only a versioned machine protocol; diagnostics go to stderr.

## macOS app

The app lives in `apps/macos/XDRemuxApp/` and links the Swift package directly rather than shelling out to the CLI — `Tests/test_swift_app_architecture.py` enforces that.

```bash
scripts/build_and_run.sh run      # build and launch
scripts/build_and_run.sh build    # build only
scripts/build_and_run.sh debug    # build the Debug configuration
scripts/build_and_run.sh verify   # swift build + swift test + the Python suites
scripts/build_and_run.sh logs     # show the last build log
scripts/build_and_run.sh clean    # remove DerivedData
```

Everything except an explicit `debug` builds Release: solving for Photographic Styles is CPU-heavy and a debug build is several times slower.

## Repository layout

| Path | Purpose |
| --- | --- |
| `Package.swift` | SwiftPM products and targets |
| `Sources/XDRemuxCore/` | Conversion core |
| `Sources/XDRemuxAppleFeatures/` | Apple-specific features |
| `Sources/XDRemuxCLI/` | Command-line parsing and entry point |
| `Sources/CoreImageRAWDiagnostics/` | Developer-only RAW diagnostics target |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI app |
| `xdremux_py/` | Cross-platform Python CLI implementation and repository-local module entry point |
| `Tests/` | Swift tests, Python policy suites, validation harnesses |
| `scripts/` | Build and acceptance scripts |

The Swift CLI has a single maintained entry point: the SwiftPM `xdremux` executable. Repository validation builds and invokes that product directly instead of maintaining a separate one-file Swift forwarding script. The Python CLI likewise has only the `xdremux-py` installed command and `python3 -m xdremux_py`, both owned by the same package.

## Debugging environment variables

None of these are needed for normal use.

| Variable | Effect |
| --- | --- |
| `XDREMUX_DISABLE_DIRECT_GAIN=1` | Disable the one-pass direct gain-map encoder |
| `XDREMUX_KEEP_GAIN_SCRATCH=1` | Keep gain-map intermediates |
| `XDREMUX_KEEP_PORTRAIT_SCRATCH=1` | Keep portrait conversion intermediates |
| `XDREMUX_ENCODING_AUDIT_DIR=<dir>` | Write encoding audit data to a directory |
| `XDREMUX_STYLE_RENDER_JOBS=<n>` | Cap Photographic Styles render concurrency |

Photographic Styles also has several `XDREMUX_RESEARCH_*` and `XDREMUX_STYLES_*` switches. They mark the output manifest as a research run and exclude it from production judgement — see the [Apple features guide](apple-features.en.md).

## Acceptance rules

Before calling a change done, run the completion gate against the final commit:

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

The receipt binds to the current HEAD, the base commit, the changed-file set, and a clean worktree. Any later commit or edit invalidates it.

Pick evidence to match the change; do not run a full real-photo matrix for a one-line documentation edit:

- Documentation only: link, command-example, and documentation-policy checks.
- CLI parsing: the matching option and output regressions.
- Conversion core: unit tests plus functional verification on real samples.
- App or helpers: build, run, or device evidence.

The plan schema and evidence requirements are in the [validation guide](validation/README.md); reusable harnesses live in `Tests/validation/`.

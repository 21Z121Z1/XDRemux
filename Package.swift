// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "XDRemux",
    defaultLocalization: "en",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        // The Rust workspace owns the product stack. Swift publishes only the
        // framework adapter needed by the Rust runtime.
        .executable(name: "xdremux-apple-adapter", targets: ["XDRemuxAppleAdapter"])
    ],
    dependencies: [],
    targets: [
        // Migration-only internal target. The canonical cross-platform core is
        // the Rust workspace; this target remains while AppleFeatures and old
        // conformance tests still depend on established Swift implementations.
        .target(
            name: "XDRemuxCore",
            path: "Sources/XDRemuxCore",
            resources: [
                .copy("Resources/Native")
            ]
        ),
        .target(
            name: "XDRemuxAppleFeatures",
            dependencies: ["XDRemuxCore"],
            path: "Sources/XDRemuxAppleFeatures",
            resources: [
                .copy("Resources/ApplePlatform")
            ]
        ),
        // Process boundary for Apple-only framework calls. The adapter is kept
        // dependency-free from migration Swift business modules so its build
        // graph matches the intended final architecture.
        .executableTarget(
            name: "XDRemuxAppleAdapter",
            dependencies: [],
            path: "Sources/XDRemuxAppleAdapter"
        ),
        // Developer-only diagnostic executable. Keeping it as a target means it
        // remains buildable with `swift build --target CoreImageRAWDiagnostics`
        // without vending it as part of XDRemux's public package product surface.
        .executableTarget(
            name: "CoreImageRAWDiagnostics",
            dependencies: ["XDRemuxCore"],
            path: "Sources/CoreImageRAWDiagnostics"
        ),
        // Developer-only parser oracle used to compare the established Swift
        // container semantics with the isolated Rust format crate. It is not a
        // public package product and is never used by the conversion hot path.
        .executableTarget(
            name: "FormatConformanceOracle",
            dependencies: ["XDRemuxCore"],
            path: "Sources/FormatConformanceOracle"
        ),
        // Developer-only metadata oracle used to prove routing, tmap, XMP, and
        // Exif UserComment patch semantics against the Rust metadata crate.
        .executableTarget(
            name: "MetadataConformanceOracle",
            dependencies: ["XDRemuxCore"],
            path: "Sources/MetadataConformanceOracle"
        ),
        .testTarget(
            name: "XDRemuxCoreTests",
            dependencies: ["XDRemuxCore"],
            path: "Tests/XDRemuxCoreTests"
        ),
        .testTarget(
            name: "XDRemuxAppleFeaturesTests",
            dependencies: ["XDRemuxAppleFeatures"],
            path: "Tests/XDRemuxAppleFeaturesTests"
        )
    ],
    swiftLanguageModes: [.v5]
)

// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "XDRemux",
    defaultLocalization: "en",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        // Swift is no longer a second XDRemux product stack. Keep only the
        // Apple platform capability library public while its implementation is
        // migrated behind Rust-owned capability contracts.
        .library(name: "XDRemuxAppleFeatures", targets: ["XDRemuxAppleFeatures"])
    ],
    dependencies: [
        .package(
            url: "https://github.com/apple/swift-argument-parser",
            from: "1.8.2"
        )
    ],
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
        // Internal process boundary for Apple-only capabilities. The protocol is
        // owned by the Rust runtime; this target reports capability facts and,
        // in later migration slices, executes narrowly scoped Apple operations.
        .executableTarget(
            name: "XDRemuxAppleAdapter",
            dependencies: ["XDRemuxAppleFeatures"],
            path: "Sources/XDRemuxAppleAdapter"
        ),
        // Migration-only executable target. It is intentionally not a public
        // package product; the Rust `xdremux` binary owns the CLI contract.
        .executableTarget(
            name: "XDRemuxCLI",
            dependencies: [
                "XDRemuxCore",
                "XDRemuxAppleFeatures",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            path: "Sources/XDRemuxCLI"
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
        ),
        .testTarget(
            name: "XDRemuxCLITests",
            dependencies: ["XDRemuxCLI"],
            path: "Tests/XDRemuxCLITests"
        )
    ],
    swiftLanguageModes: [.v5]
)

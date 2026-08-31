// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "XDRemux",
    defaultLocalization: "en",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .library(name: "XDRemuxCore", targets: ["XDRemuxCore"]),
        .library(name: "XDRemuxAppleFeatures", targets: ["XDRemuxAppleFeatures"]),
        .executable(name: "xdremux", targets: ["XDRemuxCLI"])
    ],
    dependencies: [
        .package(
            url: "https://github.com/apple/swift-argument-parser",
            from: "1.8.2"
        )
    ],
    targets: [
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

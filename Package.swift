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
        .executable(name: "xdremux", targets: ["XDRemuxCLI"]),
        .executable(name: "coreimage-raw-diagnostics", targets: ["CoreImageRAWDiagnostics"])
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
            dependencies: ["XDRemuxCore", "XDRemuxAppleFeatures"],
            path: "Sources/XDRemuxCLI"
        ),
        .executableTarget(
            name: "CoreImageRAWDiagnostics",
            dependencies: ["XDRemuxCore"],
            path: "Sources/CoreImageRAWDiagnostics"
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

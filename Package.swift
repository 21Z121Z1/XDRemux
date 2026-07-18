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
        .executable(name: "xdremux", targets: ["XDRemuxExecutable"]),
        .executable(name: "xdremux-dev", targets: ["XDRemuxDevExecutable"]),
        .executable(name: "XDRemuxSemanticHelper", targets: ["XDRemuxSemanticHelper"]),
        .executable(name: "XDRemuxHEVCEncoderHelper", targets: ["XDRemuxHEVCEncoderHelper"]),
        .executable(name: "XDRemuxStyleValidationHelper", targets: ["XDRemuxStyleValidationHelper"])
    ],
    targets: [
        .target(
            name: "XDRemuxCore",
            path: "Sources/XDRemuxCore"
        ),
        .target(
            name: "XDRemuxAppleFeatures",
            dependencies: ["XDRemuxCore"],
            path: "Sources/XDRemuxAppleFeatures",
            exclude: ["Resources"]
        ),
        .executableTarget(
            name: "XDRemuxSemanticHelper",
            path: "Sources/XDRemuxSemanticHelper"
        ),
        .executableTarget(
            name: "XDRemuxHEVCEncoderHelper",
            path: "Sources/XDRemuxHEVCEncoderHelper"
        ),
        .executableTarget(
            name: "XDRemuxStyleValidationHelper",
            path: "Sources/XDRemuxStyleValidationHelper",
            cSettings: [
                .unsafeFlags(["-fobjc-arc"])
            ],
            linkerSettings: [
                .linkedFramework("Foundation")
            ]
        ),
        .target(
            name: "XDRemuxCLI",
            dependencies: ["XDRemuxCore", "XDRemuxAppleFeatures"],
            path: "Sources/XDRemuxCLI",
            resources: [
                .process("Resources")
            ]
        ),
        .executableTarget(
            name: "XDRemuxExecutable",
            dependencies: ["XDRemuxCLI"],
            path: "Sources/XDRemuxExecutable"
        ),
        .executableTarget(
            name: "XDRemuxDevExecutable",
            dependencies: ["XDRemuxCLI"],
            path: "Sources/XDRemuxDevExecutable"
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

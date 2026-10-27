// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "XDRemux",
    defaultLocalization: "en",
    platforms: [
        .macOS("27.0")
    ],
    products: [
        // The Rust workspace owns the product stack. Swift publishes only the
        // framework adapter needed by the Rust runtime.
        .executable(name: "xdremux-apple-adapter", targets: ["XDRemuxAppleAdapter"])
    ],
    dependencies: [],
    targets: [
        .target(
            name: "XDStylesNative",
            path: "Sources/XDStylesNative",
            publicHeadersPath: "include",
            linkerSettings: [
                .linkedFramework("CoreImage"), .linkedFramework("ImageIO"),
                .linkedFramework("Metal"), .linkedFramework("CoreMedia")
            ]
        ),
        // Process boundary for Apple-only framework calls. The adapter is
        // dependency-free from product business modules so its build graph
        // matches the final architecture.
        .executableTarget(
            name: "XDRemuxAppleAdapter",
            dependencies: ["XDStylesNative", "XDCinematicNative", "XDTextureNative"],
            path: "Sources/XDRemuxAppleAdapter"
        ),
        .target(
            name: "XDTextureNative",
            path: "Sources/XDTextureNative",
            publicHeadersPath: "include",
            cSettings: [.unsafeFlags(["-fobjc-arc", "-fblocks"])],
            linkerSettings: [
                .linkedFramework("Foundation"),
                .linkedFramework("CoreGraphics"),
                .linkedFramework("CoreImage"),
                .linkedFramework("CoreVideo"),
                .linkedFramework("ImageIO"),
                .linkedFramework("Vision"),
                .linkedFramework("Metal"),
                .linkedFramework("MetalKit"),
            ]
        ),
        .target(
            name: "XDCinematicNative",
            path: "Sources/XDCinematicNative",
            publicHeadersPath: "include",
            linkerSettings: [
                .linkedFramework("AVFoundation"),
                .linkedFramework("Cinematic"),
                .linkedFramework("CoreMedia"),
                .linkedFramework("CoreVideo"),
                .linkedFramework("Foundation"),
                .linkedFramework("Metal"),
                .linkedFramework("Vision"),
            ]
        ),
    ],
    swiftLanguageModes: [.v5]
)

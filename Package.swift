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
        // Process boundary for Apple-only framework calls. The adapter is
        // dependency-free from product business modules so its build graph
        // matches the final architecture.
        .executableTarget(
            name: "XDRemuxAppleAdapter",
            dependencies: [],
            path: "Sources/XDRemuxAppleAdapter"
        )
    ],
    swiftLanguageModes: [.v5]
)

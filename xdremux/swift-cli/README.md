# Swift CLI Compatibility Entry

The production Swift CLI is defined by the root Swift Package. Run it from the repository root with:

```bash
swift build
swift run xdremux --help
swift run xdremux convert --input IMG_001.heic --output IMG_001_iso.heic
```

Existing scripts may continue to use the legacy single-file entry point:

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

The compatibility file forwards arguments to the same `xdremux` package executable. It does not contain a second conversion implementation.

Documentation:

- [Complete CLI reference](../../docs/cli.en.md)
- [中文 CLI 参考](../../docs/cli.md)
- [Development and Swift Package integration](../../docs/development.en.md)
- [Apple Photographic Styles and Portrait](../../docs/apple-features.en.md)

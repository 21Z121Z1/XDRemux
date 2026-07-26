# Legacy Swift CLI entry point

This directory exists for scripts written before XDRemux became a Swift package.
`XDRemux.swift` locates the repository root and forwards every argument to the
`xdremux` executable, so both of these run exactly the same code:

```bash
swift run xdremux convert --input IMG_001.heic
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

Prefer the first form. The forwarding shim compiles on every invocation, which
makes it noticeably slower.

The implementation lives under `Sources/`. For commands, options, and defaults,
see the [CLI reference](../../docs/cli.en.md).

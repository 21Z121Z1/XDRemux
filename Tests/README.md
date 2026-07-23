# Tests

This directory contains both SwiftPM tests and repository-level validation that
is not tied to a specific app shell.

Run the package tests from the repository root:

```bash
swift test
```

Swift target coverage is split into:

- `XDRemuxCoreTests/` for conversion models, HEIF bounds, validation, and file lifecycle.
- `XDRemuxAppleFeaturesTests/` for Apple feature contracts and byte-stable self-tests.
- `XDRemuxCLITests/` for shared convert/batch parsing, modes, defaults, and errors.

Run the existing Python regressions with:

```bash
python3 -m unittest discover -s Tests -v
```

Use this directory for tests and validation harnesses that compare converter behavior across entry points, inspect HEIF/ISOBMFF structure, or validate ISO gain-map metadata.

Recommended split:

- `Tests/fixtures/` for small synthetic metadata fixtures that are safe to commit.
- `Tests/golden/` for expected metadata snapshots, hashes, or text outputs.
- `Tests/validation/` for scripts that inspect output files without requiring a graphical app.

`Tests/validation/test_agent_completion_gate.py` verifies that the agent
completion gate rejects missing evidence and failed commands, accepts complete
plans, and invalidates receipts after `HEAD` changes.

`Tests/validation/verify_swift_cli_sample.py` is a parameterized real-sample
functional check for Swift CLI completion plans. It keeps private HEIC inputs
outside Git while verifying conversion success and the ImageIO gain-map pixel
format.

macOS app-specific UI and ViewModel tests can remain under `apps/macos/XDRemuxApp/Tests/`. Converter correctness tests should live here so they are not coupled to the app project layout.

`test_swift_apple_semantic_policy.py` pins the native role tiers, sparse-matte
policy, dynamic scaffold contract, and constrained OPPO/Vision person/hair
fusion. Run the targeted repository tests with:

```bash
python3 -m unittest -v \
  Tests.test_swift_apple_semantic_policy \
  Tests.test_python_tail_policy
```

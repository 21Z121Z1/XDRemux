# Tests

This directory is reserved for repository-level validation that is not tied to a specific app shell.

Use this directory for tests and validation harnesses that compare converter behavior across entry points, inspect HEIF/ISOBMFF structure, or validate ISO gain-map metadata.

Recommended split:

- `tests/fixtures/` for small synthetic metadata fixtures that are safe to commit.
- `tests/golden/` for expected metadata snapshots, hashes, or text outputs.
- `tests/validation/` for scripts that inspect output files without requiring a graphical app.

macOS app-specific UI and ViewModel tests can remain under `apps/macos/XDRemuxApp/Tests/`. Converter correctness tests should live here so they are not coupled to the app project layout.

`test_swift_apple_semantic_policy.py` pins the native role tiers, sparse-matte
policy, dynamic scaffold contract, and constrained OPPO/Vision person/hair
fusion. Run the targeted repository tests with:

```bash
python3 -m unittest -v \
  tests.test_swift_apple_semantic_policy \
  tests.test_python_tail_policy
```

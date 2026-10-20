# Repository automation

English | [简体中文](README.md)

Run the commands below from the repository root. These helpers build, measure,
or validate the Rust product; they do not own conversion policy.

| Directory | Responsibility |
| --- | --- |
| `ci/` | Exact-commit completion receipts and CI orchestration. |
| `validation/` | Portable Rust contracts, fixture gates, and CLI acceptance harnesses. |
| `apple/` | Apple framework and device acceptance; requires the documented platform. |
| `diagnostics/` | Read-only media inspection and comparison. |
| `performance/` | Measurements and budget checks; baselines live in `../benchmarks/`. |
| `distribution/` | Pinned native builds, arm64/macOS 27 product package relocation, archive and isolated package checks. |

```bash
python3 scripts/ci/agent_completion_gate.py --help
bash scripts/validation/check_rust_cli_smoke.sh
python3 -m unittest tests.validation.test_repository_layout -v
```

`rust-cli-core.yml` retains Linux, Windows, and macOS tests for the portable Rust core. Its product package job uses `xcode-27`, arm64, and a macOS 27 deployment target. Packaging checks every bundled Mach-O architecture and deployment target, then runs real conversions outside the checkout with only system paths. Feature-specific native and app acceptance remain separate gates.

App-only build and model-test launchers live beside the Xcode project in
[`apps/macos/XDRemuxApp/scripts/`](../apps/macos/XDRemuxApp/scripts/).
Research-specific training and export tools belong to their research area, such
as [`research/oppo_styles/tools/`](../research/oppo_styles/tools/), not this directory.

Python regression suites stay in [`tests/`](../tests/README.en.md). Versioned real
media stays in [`fixtures/`](../fixtures/README.en.md); small synthetic vectors
stay in `tests/fixtures/`. Generated reports belong in ignored `artifacts/`.

See the [validation guide](../docs/quality/evals.en.md) for complete commands and
the [testing policy](../docs/quality/testing.en.md) for evidence limits.

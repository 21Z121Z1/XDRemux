# Repository automation

English | [简体中文](README.md)

Scripts in this directory build, measure, or validate the Rust product. They do not define conversion policy. Run the examples from the repository root.

## Select the script owner

| Directory | Responsibility |
| --- | --- |
| `ci/` | CI coordination and completion receipts bound to a commit. |
| `validation/` | Portable Rust contracts, real-fixture checks, and CLI acceptance tests. |
| `apple/` | Apple framework and device acceptance on the required platform. |
| `diagnostics/` | Read-only media inspection and comparison. |
| `performance/` | Measurements and budget checks. Baselines are in `../benchmarks/`. |
| `distribution/` | Pinned native builds, arm64/macOS 27 packaging, relocation, archives, and isolated package checks. |

Start with the applicable command:

```bash
python3 scripts/ci/agent_completion_gate.py --help
bash scripts/validation/check_rust_cli_smoke.sh
python3 -m unittest tests.validation.test_repository_layout -v
```

## Understand package checks

The `rust-cli-core.yml` workflow tests the portable Rust core on Linux, Windows, and macOS. Its product package job uses `xcode-27`, arm64, and a macOS 27 deployment target.

Packaging checks each bundled Mach-O file's architecture and deployment target. It then runs real conversions outside the checkout with only system paths. Feature-specific native checks and app acceptance remain separate requirements.

## Keep tools with their owner

App build and model-test launchers belong beside the Xcode project in [`apps/macos/XDRemuxApp/scripts/`](../apps/macos/XDRemuxApp/scripts/). Research training and export tools belong to their research area, such as [`research/oppo_styles/tools/`](../research/oppo_styles/tools/).

Python regressions belong in [`tests/`](../tests/README.en.md). Real media belongs in [`fixtures/`](../fixtures/README.en.md). Small synthetic vectors belong in `tests/fixtures/`. Save generated reports in the ignored `artifacts/` directory.

The [validation guide](../docs/quality/evals.en.md) identifies reusable checks. The [testing policy](../docs/quality/testing.en.md) defines what their results can prove.

# 仓库自动化

[English](README.en.md) | 简体中文

以下命令从仓库根目录运行。这些工具负责构建、测量或验证 Rust 产品，
不拥有转换策略。

| 目录 | 职责 |
| --- | --- |
| `ci/` | 精确提交的 completion receipt 与 CI 编排。 |
| `validation/` | 可移植的 Rust 契约、fixture gate 和 CLI 验收脚本。 |
| `apple/` | Apple framework 与真机验收，需满足相应平台要求。 |
| `diagnostics/` | 只读媒体检查与比较。 |
| `performance/` | 测量与预算检查；基线存放在 `../benchmarks/`。 |

```bash
python3 scripts/ci/agent_completion_gate.py --help
bash scripts/validation/check_rust_cli_smoke.sh
python3 -m unittest tests.validation.test_repository_layout -v
```

仅用于 App 的构建与模型测试启动器，与 Xcode 工程一起放在
[`apps/macos/XDRemuxApp/scripts/`](../apps/macos/XDRemuxApp/scripts/)。
特定研究的训练与导出工具放在所属研究目录，例如
[`research/oppo_styles/tools/`](../research/oppo_styles/tools/)，不放在此处。

Python 回归测试保留在 [`tests/`](../tests/README.md)。版本化真实媒体保留在
[`fixtures/`](../fixtures/README.md)，小型合成向量保留在 `tests/fixtures/`。
生成的报告放在已忽略的 `artifacts/`。

完整命令见[验证指南](../docs/quality/evals.md)，证据边界见
[测试策略](../docs/quality/testing.md)。

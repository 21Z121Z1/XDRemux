# 仓库自动化

[English](README.en.md) | 简体中文

本目录的脚本用于构建、测量或验证 Rust 产品，不定义转换策略。示例命令从仓库根目录运行。

## 选择脚本目录

| 目录 | 职责 |
| --- | --- |
| `ci/` | CI 编排，以及绑定具体提交的完成回执。 |
| `validation/` | 可移植 Rust 契约、真实样本检查和 CLI 验收测试。 |
| `apple/` | 在要求的平台上执行 Apple 框架和真机验收。 |
| `diagnostics/` | 只读媒体检查与比较。 |
| `performance/` | 测量与预算检查，基线位于 `../benchmarks/`。 |
| `distribution/` | 固定原生依赖构建、arm64/macOS 27 打包、重定位、归档和隔离运行检查。 |

按任务选择适用命令：

```bash
python3 scripts/ci/agent_completion_gate.py --help
bash scripts/validation/check_rust_cli_smoke.sh
python3 -m unittest tests.validation.test_repository_layout -v
```

## 理解产品包检查

`rust-cli-core.yml` 工作流在 Linux、Windows 和 macOS 上测试可移植 Rust 核心。产品包任务使用 `xcode-27`、arm64 和 macOS 27 部署目标。

打包时检查每个内置 Mach-O 文件的架构和部署目标，再在检出目录之外，仅使用系统路径执行真实转换。特定功能的原生检查与 App 验收仍须分别完成。

## 工具与所属实现放在一起

App 构建与模型测试启动器放在 Xcode 工程旁的 [`apps/macos/XDRemuxApp/scripts/`](../apps/macos/XDRemuxApp/scripts/)。研究训练与导出工具放在所属研究目录，例如 [`research/oppo_styles/tools/`](../research/oppo_styles/tools/)。

Python 回归测试放在 [`tests/`](../tests/README.md)，真实媒体放在 [`fixtures/`](../fixtures/README.md)，小型合成向量放在 `tests/fixtures/`。生成报告保存到已忽略的 `artifacts/` 目录。

[验证指南](../docs/quality/evals.md)列出可复用检查。[测试政策](../docs/quality/testing.md)规定检查结果能够证明的范围。

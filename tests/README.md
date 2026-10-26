# 测试

[English](README.en.md) | 简体中文

本目录包含 Python 回归、产品与仓库规则检查、验证框架自测和小型合成样本。Rust 测试与所属 crate 放在一起。以下命令从仓库根目录运行。

## 运行 Rust 产品测试

```bash
cargo test --workspace --locked
```

Rust workspace 测试 CLI 解析、转换、批处理、Motion Photo、分类、Portrait、Styles、验证和输出安全。

执行 macOS 消费者检查时，构建 Apple 框架适配器：

```bash
swift build --product xdremux-apple-adapter
```

Swift package 对外提供适配器可执行产品，包内包含所需的原生支持目标。它不提供另一套转换产品。

## 运行 Python 仓库测试

```bash
python3 -m unittest discover -s tests -t . -v
```

这些测试覆盖仓库规则、文档、App 架构和可选研究训练代码，不实现产品转换。检查源码文本的测试提供静态证据，不能替代实际转换测试。

## 选择验证脚本

可复用的可移植检查位于 `scripts/validation/`。App 模型测试启动器位于 `apps/macos/XDRemuxApp/scripts/`。

| 脚本 | 用途 |
| --- | --- |
| `scripts/validation/check_rust_motion_photo_real_fixtures.sh` | 转换并验证真实 Motion Photo 样本集。 |
| `scripts/validation/verify_error_messages.sh` | 检查部分 CLI 帮助和错误结果。 |
| `scripts/validation/verify_batch_categorize_idempotence.sh` | 检查重复执行分类批处理的行为。 |
| `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh` | 构建并运行 macOS App 模型测试。 |

`tests/validation/` 保存完成门禁、工作流配置和仓库结构的 Python 自测，不放 shell 启动器。检查选择见[回归与真实媒体指南](../docs/quality/evals.md)。

## 选择正确的样本位置

真实 Motion Photo 样本集位于 `fixtures/`，`fixtures/SHA256SUMS` 记录文件的准确身份。

`tests/fixtures/` 用于可重新生成的小型合成输入或仅含元数据的输入。不要将真实媒体移入该目录。

## 验证完成门禁

`tests/validation/test_agent_completion_gate.py` 检查必需证据的处理和回执失效行为。[验证操作指南](../docs/validation/README.md)说明如何对已提交修改运行门禁。

## 检查文档

`tests/test_public_documentation.py` 检查双语发布规则和必需文档引用。`tests/validation/test_repository_layout.py` 检查本地链接及路径大小写。

新的规范性文档加入公开双语集合时，将其纳入文档规则检查。写作遵循[技术写作规范](../docs/style-guide.md)。

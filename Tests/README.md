# 测试

[English](README.en.md) | 简体中文

本目录包含 Swift package test、Python 仓库 policy test 和可复用 validation harness。

## Swift 测试

在仓库根目录运行全部 Swift package test：

```bash
swift test
```

主要 test target：

| Target | 范围 |
| --- | --- |
| `XDRemuxCoreTests` | 转换 model、HEIF/ISO-BMFF、Motion Photo 解析、验证、分类和文件生命周期。 |
| `XDRemuxAppleFeaturesTests` | Live Photo、摄影风格、Apple 人像、native helper 兼容性和性能契约。 |
| `XDRemuxCLITests` | CLI 解析、batch 行为、Motion Photo 路由和输出安全。 |

## Python 仓库测试

运行：

```bash
python3 -m unittest discover -s Tests -v
```

这些测试包括 Python 转换器行为，以及检查 Swift 源码、文档、fixture 或架构的仓库 policy。

源码检查类 policy test 属于 static evidence，不能替代 functional conversion test。

## Validation harness

可复用 harness 位于 `Tests/validation/`。

例如：

- `verify_swift_cli_sample.py`
- `verify_python_motion_photo_fixtures.py`
- `verify_error_messages.sh`
- `verify_batch_categorize_idempotence.sh`
- `verify_categorization_cross_implementation.py`
- `verify_categorized_batch_outputs.py`
- `verify_apple_feature_artifact_lifecycle.py`
- `verify_macos_app_model_tests.sh`

如何选择 harness 见[回归和真实样本验证](../docs/quality/evals.md)。

## 真实 Motion Photo fixture

strict Motion Photo corpus 位于 `fixtures/`，不在 `Tests/fixtures/`。

`fixtures/SHA256SUMS` 定义真实媒体文件的准确身份。

`Tests/fixtures/` 用于可以作为测试数据重新生成的小型 synthetic 或 metadata-only fixture。

## Completion gate 测试

`Tests/validation/test_agent_completion_gate.py` 测试仓库 completion gate 实现。

该测试检查必需证据处理和 receipt 失效行为。

正式验收流程见 [docs/validation/README.md](../docs/validation/README.md)。

## 文档测试

`Tests/test_public_documentation.py` 检查当前公开文档链接和双语发布规则。

增加新的规范性技术文档时，如果它属于公开双语集合，就把它加入文档 policy。

当前文档遵循[技术写作规范](../docs/style-guide.md)。

# 回归与真实样本验证

[English](evals.en.md) | 简体中文

避免"改完只靠人眼看一遍"。这里列的是仓库里可复用的验证 harness 和它们各自能证明什么。规范见[测试规范](testing.md)。

## 可复用 harness

都在 `Tests/validation/`，都能直接作为 completion gate 计划里的一条 check。

| Harness | 证明什么 |
| --- | --- |
| `verify_swift_cli_sample.py` | 拿一张真实照片跑完整转换，用 ImageIO 断言输出的 Gain Map 像素格式。`--require-compressed-primary-preserved` 额外断言主图字节没被动过；`--validate-only` 只检查已有输出，不转换 |
| `verify_error_messages.sh` | 走真实二进制检查 help 文本和错误文案：重复转换、非 ProXDR 输入、批量失败行的长度 |
| `verify_batch_categorize_idempotence.sh` | 同一个目录连跑两次 `batch --categorize`，第二次必须全部跳过而不是重新扫描自己的输出 |
| `verify_validate_only_harness.sh` | `--validate-only` 在匹配、不匹配和误用三种情况下的行为 |
| `verify_categorization_cross_implementation.py` | Swift 和 Python 两版分类结果一致 |
| `verify_categorized_batch_outputs.py` | 分类批量输出的目录结构 |
| `verify_apple_feature_artifact_lifecycle.py` | Apple 功能的中间产物按预期清理或保留 |
| `verify_macos_app_model_tests.sh` | 构建并运行 macOS App 的模型测试 |

## 挑哪一条

- 改了 Gain Map 编码或容器写入 → `verify_swift_cli_sample.py`，带上期望的像素格式。
- 改了用户能看见的文字 → `verify_error_messages.sh`。
- 改了批量枚举或续跑逻辑 → `verify_batch_categorize_idempotence.sh`。
- 改了分类判定 → 两个 categorization harness。
- 改了 App 的 ViewModel → `verify_macos_app_model_tests.sh`。

真实样本不在仓库里，路径要在计划文件里显式给绝对路径。

## 已知空白

1. 没有 golden 输出比对 —— 目前断言的是结构性质（像素格式、字节保留、退出码），不是逐位相同的输出哈希。
2. 真实样本密度不够，覆盖的机型和拍摄模式有限。
3. Apple 摄影风格的"是否真的能在 Photos 里编辑"没有自动化证据，只有容器结构层面的检查。
4. OPPO 相册的显示行为需要真机，不在任何自动化链路里。

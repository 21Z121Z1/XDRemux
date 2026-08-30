# 回归和真实样本验证

[English](evals.en.md) | 简体中文

修改需要 unit test 之外的证据时，优先使用可复用 harness。

需要哪一类证据由[测试政策](testing.md)定义。

## 版本化 Motion Photo gate

仓库的 `fixtures/` 下包含真实 Motion Photo fixture。

当前 strict gate 同时覆盖 Swift 和纯 Python 路径。转换前会检查输入文件身份，fixture 字节与 `fixtures/SHA256SUMS` 不一致时会拒绝。

fixture gate 覆盖多种 JPEG 和 HEIC/HEIF 容器布局。gate 名称和具体断言可能随着实现变化，因此 workflow 和测试源码是最终依据。

当前重要断言包括：

- Motion Photo 资源边界可解析；
- 选定封面时间可以映射到 Apple `still-image-time`；
- 输出静态照片和 MOV 共享预期 Live Photo asset identity；
- 需要时保留源 Gain Map；
- 正常 passthrough 路径保留压缩视频样本；
- 源文件有音频时保留压缩音频样本；
- 输出发布不会静默复用来源未知的有效 Live Photo pair；
- 对适用输出，macOS 验证可以通过被测试的 Apple framework 路径加载生成 pair。

## 可复用验证 harness

`Tests/validation/` 包含可复用脚本。

| Harness | 用途 |
| --- | --- |
| `verify_swift_cli_sample.py` | 运行或验证 Swift HDR 转换，并检查预期 Gain Map pixel format。 |
| `verify_error_messages.sh` | 通过真实 Swift binary 检查部分帮助和错误契约。 |
| `verify_batch_categorize_idempotence.sh` | 检查重复 categorized batch 行为。 |
| `verify_validate_only_harness.sh` | 检查 validation-only 行为。 |
| `verify_categorization_cross_implementation.py` | 对比 Swift 和 Python 分类结果。 |
| `verify_categorized_batch_outputs.py` | 检查分类目录投影。 |
| `verify_apple_feature_artifact_lifecycle.py` | 检查 Apple feature 临时产物 policy。 |
| `verify_macos_app_model_tests.sh` | 构建并运行 macOS App model test。 |
| `verify_python_motion_photo_fixtures.py` | 使用 Python 转换并验证版本化 Motion Photo corpus。 |

## 根据受影响链路选择证据

- Gain Map 编码或 HEIF 结构：使用 HDR validation harness 和代表性真实输入。
- Motion Photo parser 或 writer：使用 strict fixture gate 和针对性的 Swift/Python 测试。
- Batch provenance 或输出安全：使用 output collision 和 checkpoint 回归。
- 分类：两套实现都受影响时使用 cross-implementation 和 output-layout 检查。
- App 状态：使用 macOS App model test。
- Apple Photos 行为：除结构检查外，还需要原生框架或真机证据。

## 已知边界

没有单个 harness 可以证明全部用户可见行为。

bit-for-bit golden output 不是所有路径的通用要求，因为部分有效输出包含生成的 identifier 或依赖 framework 的编码选择。

结构验证不能证明视觉完全等价。

fixture corpus 只能证明这些文件上的行为，不能证明所有固件、设备或拍摄模式。

# 验证回归与真实媒体

[English](evals.en.md) | 简体中文

修改需要单元测试之外的证据时，使用现有验证脚本。[测试政策](testing.md)规定各类修改所需的证据。

## 检查版本化 Motion Photo

仓库的 `fixtures/` 目录包含真实 Motion Photo 测试样本。严格 Rust 检查在转换前将每个输入与 `fixtures/SHA256SUMS` 比较。摘要不一致时，检查失败。

样本集覆盖多种 JPEG 和 HEIC/HEIF 布局。具体断言以工作流和测试源码为准。适用的单元、真实样本和原生检查共同验证以下契约：

- 解析器能够识别 Motion Photo 的资源边界。
- 选定封面时间能够映射到 Apple `still-image-time`。
- 输出静态照片与 MOV 具有预期的共同 Live Photo 标识符。
- 转换要求保留源 Gain Map 时，输出保留该资源。
- 常规压缩样本直通路径保留编码视频样本和源文件中的音频样本。
- 输出发布不会静默复用来源未知的文件对。
- 适用的 macOS 检查通过被测 Apple 框架接口加载生成的文件对。

## 选择现有脚本

可移植验证脚本位于 `scripts/validation/`。App 模型测试启动器位于 `apps/macos/XDRemuxApp/scripts/`。

| 脚本 | 用途 |
| --- | --- |
| `scripts/validation/check_rust_motion_photo_real_fixtures.sh` | 通过 Rust CLI 转换全部版本化 Motion Photo，并验证两个输出成员。 |
| `scripts/validation/verify_error_messages.sh` | 通过实际 CLI 检查部分帮助和错误行为。 |
| `scripts/validation/verify_batch_categorize_idempotence.sh` | 检查重复执行带分类输出的批量转换。 |
| `scripts/validation/verify_validate_only_harness.sh` | 检查仅验证模式的行为。 |
| `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh` | 在 macOS 上构建并运行 App 模型测试。 |

根据修改影响的行为选择证据：

| 受影响的行为 | 检查 |
| --- | --- |
| Gain Map 编码或 HEIF 结构 | HDR 验证脚本和代表性真实输入。 |
| Motion Photo 解析或写入 | Rust 单元测试和严格真实样本检查。 |
| 批处理来源记录或输出安全 | 输出冲突与检查点回归。 |
| 分类 | Rust 分类测试与输出目录检查。 |
| App 状态 | App 模型测试。 |
| Photos 行为 | 结构检查，以及原生框架或真机证据。 |

## 检查当前性能报告

[`benchmark_rust_product.py`](../../scripts/performance/benchmark_rust_product.py) 增量写入 schema-v2 报告。每次被测转换记录耗时、峰值 RSS、源文件身份、完成工作量和 CLI 输出验证。哈希计算与验证位于转换计时之外。

转换或验证失败时，报告保留失败状态和已观察的样本，不生成成功汇总。批处理工作负载要求三个输入均完成转换，不得存在失败或复用项。每个发布输出都必须通过 `xdremux validate`。

[`check_performance_budget.py`](../../scripts/performance/check_performance_budget.py) 要求当前 schema-v2 报告中的每项工作负载至少完成五次测量。应用预算前，脚本从样本重新计算中位数、p95 和峰值 RSS。

报告必须标识可执行文件、`Cargo.lock` 和本次执行。执行身份包含主机、CPU 数量和显式的托管运行字段。本地未记录的字段必须为 `null`。Live Photo 样本保留同名照片和视频两个成员的身份与验证结果。

观察缺失、汇总不一致、工作量不足、验证失败，或将当前报告降级为只有汇总的 schema v1，都会使检查失败。采集在生成汇总统计前失败时，工作流摘要仍显示采集状态、样本数和错误。

## 解释历史测量

历史基线文件保留 schema v1 和原始数值。`--historical-replay` 仅用于按原有汇总契约检查归档的 v1 运行报告，不能提供当前 CI 证据。

预算检查通过不能证明性能变化的原因。速度或内存比较需要匹配的工作量、适当的同机测量，以及对限制因素的实际观察。

## 将结论限制在证据范围内

单个脚本不能证明全部用户可见行为。结构有效不等于视觉等价。样本集只能证明这些文件上的行为，不能覆盖所有固件、设备或拍摄模式。

不要要求每次转换都产生逐字节相同的基准输出。有效输出可能包含生成的标识符，或采用依赖框架的编码选择。

# 回归和真实样本验证

[English](evals.en.md) | 简体中文

修改需要 unit test 之外的证据时，优先使用可复用 harness。

需要哪一类证据由[测试政策](testing.md)定义。

## 版本化 Motion Photo gate

仓库的 `fixtures/` 下包含真实 Motion Photo fixture。

当前 strict gate 由 Rust 路径执行。转换前会检查输入文件身份，fixture 字节与 `fixtures/SHA256SUMS` 不一致时会拒绝。

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

`scripts/validation/` 包含可复用的可移植脚本；macOS App 的模型测试启动器位于 `apps/macos/XDRemuxApp/scripts/`。

| Harness | 用途 |
| --- | --- |
| `scripts/validation/check_rust_motion_photo_real_fixtures.sh` | 通过 Rust CLI 转换全部版本化 Motion Photo fixture，并验证 pair 两个成员。 |
| `scripts/validation/verify_error_messages.sh` | 通过真实 Rust binary 检查部分帮助和错误契约。 |
| `scripts/validation/verify_batch_categorize_idempotence.sh` | 检查重复 categorized batch 行为。 |
| `scripts/validation/verify_validate_only_harness.sh` | 检查 Rust validation-only 行为。 |
| `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh` | 构建并运行 macOS App model test。 |

## 根据受影响链路选择证据

- Gain Map 编码或 HEIF 结构：使用 HDR validation harness 和代表性真实输入。
- Motion Photo parser 或 writer：使用 Rust unit test 和 strict real-fixture gate。
- Batch provenance 或输出安全：使用 output collision 和 checkpoint 回归。
- 分类：使用 Rust classification test 和 output-layout 检查。
- App 状态：使用 macOS App model test。
- Apple Photos 行为：除结构检查外，还需要原生框架或真机证据。

## 当前产品测量报告

`scripts/performance/benchmark_rust_product.py` 增量写入 schema-v2 报告。
每次被测转换保留耗时、峰值 RSS、源文件身份、完成工作量和标准 CLI 输出验证。
验证与哈希计算位于转换计时之外。转换或验证失败时，报告保留失败状态和已观察的样本，
不会生成成功汇总。batch workload 要求三个输入均完成转换，没有失败或复用项，
并通过 `xdremux validate` 验证每个发布输出。

`scripts/performance/check_performance_budget.py` 要求当前 schema-v2 报告中，
每个 workload 至少有五次完成的测量。它先从样本重算 median、p95 和峰值 RSS，
再应用现有限制。缺失观察、汇总不一致、工作量不足、验证失败，
以及把当前报告降级为只有汇总的 v1，都会使检查失败。

历史 baseline 文件保留 schema v1 及其原始参考值。`--historical-replay` 仅用于
按原有汇总合同检查归档的 v1 run 报告，该模式不提供当前 CI 证据。
预算通过不构成有因果依据的性能比较。速度或内存结论需要匹配的工作、
适当的同机测量，以及对限制因素的实际观察。

## 已知边界

没有单个 harness 可以证明全部用户可见行为。

bit-for-bit golden output 不是所有路径的通用要求，因为部分有效输出包含生成的 identifier 或依赖 framework 的编码选择。

结构验证不能证明视觉完全等价。

fixture corpus 只能证明这些文件上的行为，不能证明所有固件、设备或拍摄模式。

# 编码质量审计摘要：2026-07-18

[English](encoding-quality-pareto-20260718.summary.en.md) | 简体中文

[原始审计](encoding-quality-pareto-20260718.md)比较了 2026-07-18 的质量、编码载荷大小和编码行为。表格属于历史证据。本摘要没有重新测量，也不声明当前编码器的默认值。

## 测量范围

审计测量当时实现主动编码的载荷。普通 UHDR/LHDR 的主图 HEVC 载荷按字节复制，不属于编码质量比较范围。

| 载荷 | 历史参考与指标 |
| --- | --- |
| 人像 `src.image` 主图 | 解码后的源 JPEG；全分辨率 RGBA 的 PSNR、MAE 和 SSIMULACRA2。 |
| UHDR/LHDR/人像 Gain Map | 解码源图或重建栅格；码值 MAE、P99 和 PSNR。 |
| Styles Linear Thumbnail | 无损生成的 PNG；载荷大小和 SSIMULACRA2。 |
| 中性 Style Delta Map | 常量参考；空间常量性和编码样本大小。 |
| 语义掩膜 | 原始辅助缓冲区；码值误差和阈值交并比。 |
| 视差 | 原始辅助缓冲区；Float16 绝对误差。 |

载荷大小指受影响的 HEIF 项目数据，不是无关的整文件大小变化。

## 历史选择与主要限制

审计为当时测试的实现选择了以下设置：

| 载荷 | 选定设置 | 记录中的依据 |
| --- | --- | --- |
| 人像主图 | 质量 `0.90` | 三个真实样本；`0.95` 增加 58.5% 载荷，平均 SSIMULACRA2 增加 1.79 分。 |
| Gain Map | 质量 `0.90` | 九个真实样本；`0.95` 增加 75.3% 载荷，而绝对误差的下降更小。 |
| Gain Map 图块 | 512 像素 | 1024 图块仅为普通样本节省 0.23%，且未通过人像资源图等价性检查。 |
| Linear Thumbnail | 质量 `0.85` | 三张生成的缩略图；比 `0.95` 少 54.3% 载荷，平均 SSIMULACRA2 低 0.294 分。 |
| 中性 Delta Map | 质量 `0.30` | 512 像素常量图块重复 30 次；IDR 样本为 151 字节，`1.0` 时为 202 字节。 |
| 语义/视差资源 | 由 ImageIO 管理编码 | 所测主图质量请求没有改变其载荷或解码栅格。 |

审计还发现了填充位置缺陷：高度从 1532 填充到 1536 时，Gain Map 偏移四行。在高细节人像样本上，修复使质量 `0.90` 的 MAE 从 8.881 降至 0.850，P99 从 60 降至 4。

某些质量请求选择了同一个 VideoToolbox 档位。所测 ImageIO Gain Map 路径在请求 `0.80`、`0.90`、`0.95` 和 `1.0` 时也产生了相同输出。因此，数值质量参数不一定能改变特定载荷。

## 如何用于当前工作

准确的历史测量值和样本数量以原表为准。没有新测量时，不要将这些结果推广到其他编码器、操作系统、图像类别或资源类型。

当前设置见[运行时](../../crates/xdremux-runtime/src/lib.rs)、[Styles 运行时](../../crates/xdremux-runtime/src/apple_styles.rs)和[编码请求模型](../../crates/xdremux-codec/src/model.rs)。[引擎策略](../../crates/xdremux-engine/src/product_policy.rs)定义当前 Gain Map 的通道和色度选择。

原始证据覆盖 macOS VideoToolbox/ImageIO、解析检查和解码栅格，不能证明物理设备上的 OPPO 相册或 Apple Photos 行为。收集新证据时遵循[验证指南](README.md)，并单独记录设备观察。

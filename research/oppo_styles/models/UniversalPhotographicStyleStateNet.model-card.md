# UniversalPhotographicStyleStateNet 模型卡

[English](UniversalPhotographicStyleStateNet.model-card.en.md) | 简体中文

`UniversalPhotographicStyleStateNet` 是用于单图摄影风格重建研究的 Core ML `mlprogram`。Rust 转换流程不会选择该模型，旧 Swift 开关 `XDREMUX_RESEARCH_UNIVERSAL_STYLE_COREML_MODEL` 已退役。

模型预测候选 Key 1、GTC、光照图 `c` 和 `d`，以及标量值。这些预测不是已验证的拍摄事实。

## 模型包接口

原始模型包记录为 FP16 权重，共 10,501,091 个训练参数。

| 方向 | 名称 | 形状与含义 |
| --- | --- | --- |
| 输入 | `features` | `1 × 9 × 256 × 256` 主图特征。 |
| 输入 | `metadata` | `1 × 16` 元数据值。 |
| 输入 | `metadata_mask` | `1 × 16` 可用性掩码。 |
| 输出 | `key1` | `1 × 34560`，含填充的网格为 `12 × 12 × 8 × 10 × 3`。 |
| 输出 | `key1_log_variance` | `1 × 240` 不确定度值。 |
| 输出 | `gtc` | `1 × 516` 归一化字节预测。 |
| 输出 | `light_maps` | `1 × 2048` 光照图数值。 |
| 输出 | `scalars` | `1 × 6` 标量预测。 |

六个标量位置依次为 TagH、IOriginalRangeMin、IOriginalRangeMax、IGain、Tag4 和 Tag5。序列化时根据方向选择 `12 × 9` 或 `9 × 12` 有效网格，得到 51,840 字节的 Key 1 候选数据。

| 模型包文件 | SHA-256 |
| --- | --- |
| `Manifest.json` | `c31a42263e9e23a378edc04371340794a6e27d9425d9755e9892460d217cd7be` |
| `Data/com.apple.CoreML/model.mlmodel` | `84c51a998dc293165ec202215bce8d0412e46776d9a5e4b28aa87cdc13798d4c` |
| `Data/com.apple.CoreML/weights/weight.bin` | `7acd3ed2478aa28e90870140d5c1aaeaa9d929acbe869490c48419495fe7107a` |

字节身份检查用于确定受测模型包。[CoreMLProbe.swift](../CoreMLProbe.swift) 使用 `.cpuOnly`，以合成输入检查编译、特征形状和输出有限性，不测量原生风格精度或 Neural Engine 分配情况。历史运行允许 `.all`，同样不能证明实际使用了 Neural Engine。

## 历史数据集与精度

以下测量来自原模型卡，尚不能仅凭公开模型包复现。

原始数据集包含 603 个可用 iPhone 样本，来自 472 个拍摄会话。报告称各集合按会话隔离，训练集、校准集和留出集分别有 417、89、97 个样本，设备为 iPhone 16、16 Pro、17 和 17 Pro。

私有原图、缓存、训练检查点和逐图报告不在 Git 中。公开模型包不能独立证明所述数据拆分及其来源。

| 历史指标 | 记录值 |
| --- | ---: |
| 通用模型留出集 Key 1 归一化 MAE | `0.82233` |
| 未训练单位映射的 Key 1 归一化 MAE | `0.93275` |
| 风格化与无风格成对集成模型的 Key 1 归一化 MAE | `0.78123` |
| 光照图 MAE | `0.52812` |
| 标量 MAE | `0.42440` |
| 辅助无风格图像 MAE | `0.07256` |

这些观察支持保留单输入研究方案，但不能证明它可以替代精度更高的成对模型，也不能证明任意图像上的原生等价性。

## 历史 OPPO 覆盖率

一次没有原生标签的试验记录了 213 张不同原图，来自 Find X6 Pro、X7 Ultra、X8 Ultra、X9 和 X9 Ultra。样本包含 141 个 HEIC、62 个 JPEG 和 10 个 DNG 文件，没有报告解码失败。

根据 iPhone 校准集不确定度设置的阈值接受了 187/213 个候选（`87.8%`）。这是覆盖率结果，不是 Apple 渲染精度。

旧模型卡还记录过 PNG、TIFF、WebP 和 AVIF 解码试验。DNG 使用内嵌预览，却错误地标记 RAW 可用。维护中的推理实现只在提供已解码旁车文件时标记输入可用。发布的检查点没有接入 CIRAW 线性张量，因此历史 DNG 结果不能证明模型使用了 RAW 条件输入。

## 历史耗时与转换比较

| 观察项 | 记录值 |
| --- | ---: |
| 本地 MPS 模型 p95 | `29.2 ms` |
| 本地 MPS 端到端 p95 | `0.693 s` |
| 本地 MPS 端到端最大值 | `1.066 s` |
| 单张 X9 Ultra HEIC 的 Core ML 预热后 p95 | `21.9 ms` |
| 该输入的 Key 1 转换平均绝对差异 | `9.04e-5` |
| 该输入的 Key 1 转换最大差异 | `0.001175` |

这些时间取决于测试机器。单输入转换比较不能证明一般延迟、精度或 Neural Engine 执行情况。

## 尚未验证的行为

引用的历史试验均未完成 Apple Photos 导入、编辑、保存和重新打开。现有证据不能证明阈值接受的候选在原生渲染中优于单位映射或约束求解器。

有限值候选生成、原生框架解析、原生渲染和 Photos 可逆编辑是不同结论。一个层级通过不代表下一个层级通过。维护工具见[研究入口](../README.md)，实验对照要求见[方法说明](../methodology.md)。

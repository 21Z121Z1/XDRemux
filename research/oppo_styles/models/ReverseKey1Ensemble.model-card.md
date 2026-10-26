# ReverseKey1Ensemble 模型卡

[English](ReverseKey1Ensemble.model-card.en.md) | 简体中文

`ReverseKey1Ensemble` 根据风格化图像和无风格参考图预测摄影风格 Key 1 资源。它是保留的 Core ML 研究模型，Rust CLI 不加载这个模型包，也不将它用作默认 `constrained-solver`。

## 模型接口

| 项目 | 值 |
| --- | --- |
| 格式 | Core ML `mlprogram` |
| 权重精度 | FP16 |
| 输入 | `features`，形状为 `1 × 12 × 256 × 256` |
| 输出 | `key1`，形状为 `1 × 34560` |
| 含填充的输出网格 | `12 × 12 × 8 × 10 × 3` |
| 实际序列化网格 | 横向为 `12 × 9 × 8 × 10 × 3`，纵向为 `9 × 12 × 8 × 10 × 3` |
| 序列化 Key 1 | 25,920 个 Float16 数值，共 51,840 字节 |
| 保留模型的候选混合权重 | `0.625` |

[特征构造器](../../../xdremux_py/apple_reverse_key1_training.py)组合风格化 RGB、无风格 RGB、两者的 RGB 差值及 YCbCr 差值。集成模型组合小型、按设备配置条件化的基线模型和较大的多尺度候选模型。

输出张量包含填充区域。[Core ML 预测脚本](../tools/predict_reverse_key1_coreml.py)在序列化前，根据图像显示方向选择有效网格。

## 运行研究预测器

预测器需要 macOS、`coremltools` 和研究用 Python 依赖。输入必须同时包含风格化图像及其无风格参考图。脚本请求 `ComputeUnit.ALL`，允许 Core ML 选择计算单元，但不能证明实际使用了 Neural Engine。

两个输出文件都应使用新路径。这个诊断脚本直接写入 Key 1 数据和 JSON 报告，可能覆盖已有文件。

从仓库根目录运行：

```bash
python -m research.oppo_styles.tools.predict_reverse_key1_coreml \
  --model research/oppo_styles/models/ReverseKey1Ensemble.mlpackage \
  --styled /data/styled.png \
  --unstyled /data/unstyled.png \
  --output /data/key1.bin \
  --report /data/key1-report.json
```

结果是 Key 1 数据和诊断报告，不是转换后的 HEIC。`--reference-key1` 可增加与指定参考数据的数值比较。推理失败时，该脚本不提供产品级回退。

## 模型包身份

以下哈希标识保留模型包的字节内容：

| 模型包文件 | SHA-256 |
| --- | --- |
| `Manifest.json` | `8be470b08161e63d533dc28d9bb500226961adf0db2c1b4cd0c4e9d3b36ea6ef` |
| `Data/com.apple.CoreML/model.mlmodel` | `2759cc216f91ebe5fd5e1e5c3ba55c4d714080435a7b21f91efac0894ecd6a2a` |
| `Data/com.apple.CoreML/weights/weight.bin` | `88bb5c96b5a1e18a818267bed9cc7c0a0a998196ddc073c9c85b1f2507697f2c` |

模型变化后，记录新哈希并单独评估新模型包。不能将这些身份信息复制给替换后的文件。

## 证据边界

历史研究在有限 OPPO 图像和风格响应对比中评估过该模型。这些观察不能证明：

- 逐字节复现 Apple 私有生成器；
- 在未见过的设备、镜头或拍摄模式上保持相同精度；
- 完成 Apple Photos 导入、编辑、保存和重新打开的全过程；
- 每次预测都使用 Neural Engine。

除非另有产物明确发布，公开模型包不包含训练数据和检查点。导出与评估脚本位于 [tools](../tools/)。这些脚本的存在不代表 Rust CLI 提供模型选择选项。

接入产品时，必须同时修改主实现、默认行为文档和产品测试。当前产品边界见 [Apple 功能文档](../../../docs/apple-features.md)。

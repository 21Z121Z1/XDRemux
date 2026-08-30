# ReverseKey1Ensemble 模型卡

[English](ReverseKey1Ensemble.model-card.en.md) | 简体中文

`ReverseKey1Ensemble` 是摄影风格 Reverse Key 1 链路的可选研究模型。

它不是默认 style-data producer。

## 启用方式

调用方必须通过当前摄影风格链路使用的研究配置显式提供模型路径。

不要把这个模型描述为默认 `constrained-solver`。

## 模型契约

| 项目 | 值 |
| --- | --- |
| 格式 | Core ML `mlprogram` |
| 权重精度 | FP16 |
| 输入名 | `features` |
| 输入形状 | `1 × 12 × 256 × 256` |
| 输出名 | `key1` |
| 输出形状 | `1 × 34560` |
| 实际序列化 lattice | `12 × 9 × 8 × 10 × 3` |
| 序列化 Key 1 大小 | 51,840 bytes Float16 数据 |
| Core ML compute units | `.all` |

runtime 根据 styled、unstyled 图像数据及其差值构建 feature channel。

ensemble 组合小型 profile-conditioned baseline 和更大的 multiscale candidate。当前 candidate blend weight 是 `0.625`。

`.all` 允许 Core ML 选择可用 compute unit，但不能证明某次推理实际运行在 Neural Engine。

## 验证边界

该模型作为研究 fast path，在有限 OPPO 样本和 style-response 对比上进行过评估。

这些结果不能证明：

- 对 Apple 私有 producer 的 bit-exact 复现；
- 在未见设备、镜头或拍摄模式上具有相同质量；
- 完成 Apple Photos 导入、编辑、保存、重新打开全流程；
- 每次推理都运行在 Neural Engine。

研究模型或 proxy path 失败时，当前研究 envelope 可以回退，因此可选模型不是强制产品依赖。

## 文件身份

模型 package 包含：

- `Manifest.json`；
- `Data/com.apple.CoreML/model.mlmodel`；
- `Data/com.apple.CoreML/weights/weight.bin`。

实验需要准确模型身份时，使用对应仓库或 release 的 hash。模型 package 变化后，不要把旧 hash 复制到新模型卡。

## 训练和导出

仓库的 `scripts/` 下包含模型导出和评估脚本。

除非独立 artifact 明确发布，否则训练数据和 checkpoint 不属于公开模型 package。

研究模型的修改不能改变文档中的默认摄影风格 producer，除非 CLI 默认值和 production test 在同一 revision 中一起改变。

当前产品边界见 [Apple 功能文档](../docs/apple-features.md)。

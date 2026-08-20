# ReverseKey1Ensemble

这是 XDRemux 的备选研究模型，用原生 Apple `styled + unstyled` 缩略图预测结构化
Reverse Key 1。它不是默认生产路径；调用方必须显式设置：

```bash
XDREMUX_RESEARCH_REVERSE_KEY1_COREML_MODEL="$PWD/Models/ReverseKey1Ensemble.mlpackage"
```

## 模型契约

- 格式：Core ML `mlprogram`，FP16 权重。
- 输入：`features`，形状 `1 × 12 × 256 × 256`。Swift runtime 从 styled、unstyled、
  差值和亮度/色度差构建通道。
- 输出：`key1`，形状 `1 × 34560`。runtime 取有效的 `12 × 9 × 8 × 10 × 3`
  lattice，并确定性序列化为 51,840 字节 Float16 key1。
- 结构：小型 profile-conditioned baseline 与较大 multiscale candidate 的固定融合；
  candidate 权重为 `0.625`。
- 运行配置：Core ML `computeUnits = .all`。这允许系统调度 Neural Engine，但不证明
  某次推理实际运行在 Neural Engine。

## 已验证范围

### 单图 self-pair 研究分支（未 promotion）

训练管线支持显式 `--single-image-self-pair` 模式：将同一张 styled
primary 缩略图复制为第二个输入，仍使用原生 key1 标签与固定的
417/89/97 session split。该模式会在 checkpoint/report 中写入
`inputMode=single_image_self_pair`，不能与真实 disabled-unstyled paired
模型混淆，也未接入默认 runtime。v3 small 2-epoch 与 v4 multiscale 2-epoch
短程候选的 held-out normalized MAE 分别为 `0.80226` 与 `0.78751`；两者都
劣于未微调 self-pair baseline `0.78220`，因此保留为可复现实验而拒绝 promotion。

- 五张额外 OPPO 原生 HEIC 的离线端到端转换为 `3.80–5.62` 秒，均通过 XDRemux
  结构和摄影风格 metadata 校验。
- 四个具有完整 Neutrino A/B 的 OPPO 场景中，快速语义代理与完整 renderer 对模型
  相对 identity 的选择方向一致：拒绝一个退化结果，接受三个改善结果。
- 模型或代理失败时，20 秒有界路径回退到 identity，不进入分钟级 constrained solver。

这些结果不等同于真实 Apple Photos 的导入、编辑、保存再打开验收，也不证明未见过的
OPPO 设备、镜头和处理模式都能获得同等质量。

## 文件身份

| 文件 | SHA-256 |
| --- | --- |
| `Manifest.json` | `8be470b08161e63d533dc28d9bb500226961adf0db2c1b4cd0c4e9d3b36ea6ef` |
| `Data/com.apple.CoreML/model.mlmodel` | `2759cc216f91ebe5fd5e1e5c3ba55c4d714080435a7b21f91efac0894ecd6a2a` |
| `Data/com.apple.CoreML/weights/weight.bin` | `88bb5c96b5a1e18a818267bed9cc7c0a0a998196ddc073c9c85b1f2507697f2c` |

导出代码位于 `scripts/export_reverse_key1_coreml.py`。训练数据和 checkpoint 未随模型
发布；不要把这个模型的输出描述为对 Apple 私有 producer 的 bit-exact 复现。

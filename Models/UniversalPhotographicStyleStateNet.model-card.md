# UniversalPhotographicStyleStateNet（备选方案）

这是 XDRemux 的单图端到端摄影风格状态研究模型。它接受一张主图和带缺失掩码的
元数据，输出 key1、GTC、c/d light maps、capture scalars 和置信度。它不依赖运行时
unstyled 图；训练中的 unstyled 只作为辅助监督。

此模型目前不是默认生产路径，也不能替代原生响应/Photos 验收。研究开关
`XDREMUX_RESEARCH_UNIVERSAL_STYLE_COREML_MODEL` 只授权 key1 进入现有有界语义代理；
低置信度输入 fail closed 到 identity，GTC/c/d/scalars 暂时只保存为候选状态。

## 模型契约

- 格式：Core ML `mlprogram`，FP16 权重，10,501,091 个训练参数。
- `features`：`1 × 9 × 256 × 256`，依次为 RGB、luma、Cb、Cr、log-luma、
  横向/纵向 luma gradient。
- `metadata` / `metadata_mask`：各 `1 × 16`。当前 iPhone 语料中没有正例变化的
  RAW、gain-map、bit-depth、alpha 字段会自动禁用，不会把第一次出现的值放大成
  未受监督的 FiLM 激活。
- `key1`：`1 × 34560`，对应 padded `12 × 12 × 8 × 10 × 3`；writer 按方向取
  有效 `12 × 9` / `9 × 12` 并确定性序列化为 51,840 字节 Float16。
- `key1_log_variance`：`1 × 240`，用于不确定度和 fail-closed gate。
- `gtc`：`1 × 516`，对应完整 native Tag3 混合二进制资源的归一化字节预测。
- `light_maps`：`1 × 2048`，对应 c/d 两张 `32 × 32` Float16 map。
- `scalars`：`1 × 6`，顺序为 TagH、IOriginalRangeMin、IOriginalRangeMax、IGain、
  Tag4、Tag5，并限制在训练语料的原生分布边界内。
- Core ML 使用 `computeUnits = .all`；系统可以调度 Neural Engine，但这不证明某次
  推理实际运行在 ANE。训练使用 Apple GPU 的 MPS。

## iPhone 监督与留出

- 603 个可用 iPhone native 样本，472 个独立拍摄会话。
- 按拍摄会话拆分：417 train、89 calibration、97 heldout；不存在同一会话跨集合泄漏。
- 训练包含 iPhone 16、16 Pro、17、17 Pro。单张样本只适合作为字节回归金样，不能
  代替完整会话 heldout。
- heldout key1 normalized MAE 为 `0.82233`，未训练 identity 基线为 `0.93275`；
  light maps 为 `0.52812`，scalars 为 `0.42440`，辅助 unstyled MAE 为 `0.07256`。
- paired styled+unstyled ensemble 的 key1 指标仍更好（`0.78123`），因此这是一条用
  少一个运行时输入换取通用性和速度的备选路径，不是精度已全面胜出的结论。

## 非 iPhone 覆盖与性能

- 213 张不重复 OPPO 原图，覆盖 Find X6 Pro、X7 Ultra、X8 Ultra、X9、X9 Ultra；
  141 HEIC、62 JPEG、10 DNG，0 个解码失败。
- 用 iPhone calibration p95 不确定度作为试跑阈值，187/213（87.8%）可进入
  fast-path candidate；这只证明 label-free 覆盖率，不证明 Apple 响应正确率。
- 本机 MPS 批量测试：模型 p95 `29.2ms`，包含解码、EXIF、哈希和状态构造的 p95
  `0.693s`，最慢 `1.066s`。
- PNG、TIFF、WebP、AVIF 也完成真实解码/状态构造测试；DNG 当前使用嵌入预览作为
  主图并标记 RAW 可用，尚未把 CIRAW 线性张量接进这个 checkpoint。
- Core ML 对真实 OPPO X9 Ultra HEIC：warm p95 `21.9ms`；key1 平均绝对转换误差
  `9.04e-5`，最大 `0.001175`。

这些结果没有完成 Apple Photos 导入、编辑、保存再打开验收，也没有证明 87.8% 的
候选全部比 identity 或 constrained solver 更好。运行时仍需少量原生响应 probe。

## 文件身份

| 文件 | SHA-256 |
| --- | --- |
| `Manifest.json` | `c31a42263e9e23a378edc04371340794a6e27d9425d9755e9892460d217cd7be` |
| `Data/com.apple.CoreML/model.mlmodel` | `84c51a998dc293165ec202215bce8d0412e46776d9a5e4b28aa87cdc13798d4c` |
| `Data/com.apple.CoreML/weights/weight.bin` | `7acd3ed2478aa28e90870140d5c1aaeaa9d929acbe869490c48419495fe7107a` |

训练、预测、OOD 评估和导出入口分别是：

- `scripts/train_universal_photographic_style.py`
- `scripts/predict_universal_photographic_style.py`
- `scripts/evaluate_universal_photographic_style_ood.py`
- `scripts/export_universal_photographic_style_coreml.py`

训练原图、缓存、checkpoint 和逐样本 OOD 报告不随仓库发布。

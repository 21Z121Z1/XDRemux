# UniversalPhotographicStyleStateNet — 研究候选

[English](UniversalPhotographicStyleStateNet.model-card.en.md) | 简体中文

保留的 Core ML `mlprogram` 不是产品 provider。旧 Swift 环境变量
`XDREMUX_RESEARCH_UNIVERSAL_STYLE_COREML_MODEL` 已退役，canonical Rust 转换不读取它。
GTC、c/d 与 scalars 都是预测候选，不能当作已验证拍摄事实。

## 字节契约

原模型记录为 FP16 权重、10,501,091 个训练参数。`features` 为 `1 × 9 × 256 × 256`；
`metadata`、`metadata_mask` 各为 `1 × 16`。输出分别为 `key1`（`1 × 34560`，对应 padded
`12 × 12 × 8 × 10 × 3`）、`key1_log_variance`（`1 × 240`）、`gtc`（`1 × 516` normalized bytes）、
`light_maps`（`1 × 2048`）和 `scalars`（`1 × 6`）。scalars 顺序为 TagH、IOriginalRangeMin、
IOriginalRangeMax、IGain、Tag4、Tag5。只有与方向匹配的 `12 × 9` 或 `9 × 12` 有效 lattice
被序列化为 51,840-byte key1 candidate。

| Package 文件 | SHA-256 |
| --- | --- |
| `Manifest.json` | `c31a42263e9e23a378edc04371340794a6e27d9425d9755e9892460d217cd7be` |
| `Data/com.apple.CoreML/model.mlmodel` | `84c51a998dc293165ec202215bce8d0412e46776d9a5e4b28aa87cdc13798d4c` |
| `Data/com.apple.CoreML/weights/weight.bin` | `7acd3ed2478aa28e90870140d5c1aaeaa9d929acbe869490c48419495fe7107a` |

自动化 byte identity 和 synthetic forward tests 与下面的历史精度、性能观察分开。
允许 `.all` compute units 不能证明 Neural Engine 执行。

## 历史观察，不是本轮重新测得的结果

原 model card 记录了 603 个可用 native iPhone 样本、472 个 capture session，417 train、
89 calibration、97 heldout，覆盖 16、16 Pro、17、17 Pro，并报告按 session 隔离。
底层私有原图、cache、训练 checkpoint 和逐图报告不在 Git，仅靠此 package 无法独立重跑
这些统计和 provenance 结论。

记录的 heldout key1 normalized MAE 为 `0.82233`，未训练 identity 为 `0.93275`，更好的
paired styled+unstyled ensemble 为 `0.78123`。light-map MAE `0.52812`，scalar MAE
`0.42440`，辅助 unstyled MAE `0.07256`。这些数字说明单输入研究方案值得保留，但不能
据此替换更准确的 paired 路径，或宣称任意图像都具备 native equivalence。

label-free OPPO 试验记录了来自 Find X6 Pro、X7 Ultra、X8 Ultra、X9、X9 Ultra 的 213 张
不同原图：141 HEIC、62 JPEG、10 DNG，没有 decode failure。由 iPhone calibration
uncertainty 得出的阈值接受了 187/213（`87.8%`）候选；这是 coverage，不是 Apple-response accuracy。

历史本地 MPS 测量为 model p95 `29.2 ms`、end-to-end p95 `0.693 s`、最大 `1.066 s`。
单张 X9 Ultra HEIC 的 Core ML 试验记录 warm p95 `21.9 ms`，key1 转换 mean absolute
差异 `9.04e-5`、最大 `0.001175`。这些机器相关时间和单输入 parity 不是普遍 latency、
quality 或 ANE-placement 保证。

旧 card 还记录了 PNG/TIFF/WebP/AVIF decode trial。DNG 使用 embedded preview，却错误
标记 RAW 可用。维护中的 inference 现在只在提供 decoded sidecar 时设置模态 availability；
发布 checkpoint 并未接入 CIRAW linear tensor，所以旧 DNG coverage 不能证明 RAW-conditioned inference。

这些历史试验均没有完成 Apple Photos import、edit、save、reopen，也没有证据证明所有
接受的候选在 native response 上优于 identity 或 constrained solver。有限值候选生成、
native framework parse、native response 和 Photos reversible editing 始终是不同证据层级。

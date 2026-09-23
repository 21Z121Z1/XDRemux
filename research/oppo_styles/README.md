# 摄影风格重建研究

[English](README.en.md) | 简体中文

本目录是维护中的研究工具，不是 Rust 产品 provider。预测的 `key1`、GTC、light maps
和 capture scalars 都是候选数据，不是已经恢复的拍摄事实。模型、离线指标、parser
通过或不确定度阈值，都不能证明原生 consumer 等价或 Apple Photos 可逆编辑。

## Ownership 与输入

`training.py` 负责单主图和带掩码的多模态研究模型，复用 canonical
`xdremux_py/apple_reverse_key1_training.py` 中的 paired/self-pair 研究原语，不建立第二套产品转换器。
`inference.py` 读取输入、生成有限数值候选；`public_pretraining.py` 使用公开图像和
解析可得的合成 affine 标签，而不是 Apple producer 标签。

整数 RGB 使用 `[0, 255]`，浮点 RGB 必须在 `[0, 1]`。解释取决于 dtype，不能根据
图像最大值猜测：最大字节值为 1 的暗图不能被当作全白图。输入为已应用方向的
`3 × 256 × 256` RGB；九个主图特征是 RGB、luma、Cb、Cr、log-luma 和两个梯度。

可选 linear RGB sidecar 是 `[0, 1]` 中已应用方向的 scene-linear 数据；gain-map
sidecar 是已经解码的正数线性曝光比，1 表示中性。它们均有 availability mask 和
SHA-256 身份。DNG embedded preview 是 RGB，不是 decoded RAW；marker 字符串也不是
decoded gain map。历史模型特征名 `has_raw` / `has_gain_map` 表示实际提供了对应解码模态，
文件后缀和 marker 搜索不能设置它们。发布的历史 checkpoint 并未学习所有可选模态的正例。

metadata 使用独立的 missingness mask。布尔 EXIF 值不转换为数值拍摄测量。缺失的
scalar 标签既不参与 loss，也不优化该任务的不确定度；其评估指标为 `null`，不是零。

## 数据集与评估契约

universal dataset v2 固定 source manifest、label archive 和每个 cached sample 的身份。
校验针对实际解码的字节，同时检查 shape、mask、有限值，拒绝重复 source identity、
错误 label index 和 capture-session 泄漏。准备阶段在 metadata 提取前后校验原始 source。
旧版 v1 应从已验证输入重新准备，不能默默补齐缺失身份后声称已验证。

GTC 标签保留完整的 516-byte mixed-layout Tag3 字节。将其当成平坦 Float16 数组是错误的。
另一方面，预测结果长度为 516 字节，也不能证明其中结构字段构成有效 native GTC。

`base`、`multiscale_large`、`multimodal_large` 都是研究模型变体。warm start 必须包含
全部 learned parameters，保留目标语料统计量，并将新增 modality channel 初始化为零。
`--resume` 只表示绑定同一 manifest 的 weights-only continuation，不恢复完整 optimizer/RNG。
所有运行（包括 continuation）必须使用新的输出目录；不复用已有 checkpoint，也不跟随输出符号链接。
预测、loss、gradient 或选择指标出现非有限值时立即失败，不能沿用旧 `best.pt`。
checkpoint 在新运行目录内完成暂存后才原子替换；manifest 在拟合前绑定哈希，在发布证据前重新检查。
checkpoint 由 calibration 选择，随后才进行 heldout evaluation。
`consumerProxyRMSE8` 使用逐像素 squared residual，而不是逐图 MAE 的平方；encoded-RGB
quadratic proxy 只是训练 regularizer，不是 Apple renderer。

public pretraining 至少需要三个独立 source image。训练前固定 source-disjoint 的
train/calibration/heldout，冻结所选权重后才访问 heldout。历史实验曾用名为“heldout”的
集合选模型，这些数字只能算 selection-set metric，不能重新标为独立 heldout evidence。

collector 保存发布者提供的 license metadata 和 attribution，allowlist 不等于独立法律判定。
固定 revision 的 scikit-image 图像在解码前验证 Git blob identity。Commons original-file
SHA-1 与 resized download 的 SHA-256 对应不同字节，不能直接当成同一对象比较；tensor
另有独立 hash。已有 corpus receipt 和 tensor 不会被覆盖。

## 命令

用 `python -m pip install '.[training]'` 安装研究依赖。
私有 native 原图、cache、checkpoint 和原始逐图报告不随仓库发布。

```bash
python -m research.oppo_styles.train prepare --native-manifest /data/native/manifest.json --output /data/universal-v2
python -m research.oppo_styles.train train --manifest /data/universal-v2/manifest.json --output /data/run
python -m research.oppo_styles.pretrain collect --curated-only --output /data/public.json --image-directory /data/public-images
python -m research.oppo_styles.pretrain train --manifest /data/public.json --output /data/synthetic-run
python -m research.oppo_styles.predict --input /data/photo.heic --checkpoint /data/run/best.pt --output /data/candidate.zip
```

预测通过同一文件系统内的 atomic no-clobber link 发布一个完整 ZIP。ZIP 内包含有限值
resource bytes 和 hash-bound report，**不是 HEIC carrier**。staging 失败不会发布文件，
也不修改输入。source 与 decoded sidecar 的身份会被记录，不会伪造 hardware、person、
scene 或 capture fields。

只加载来源已核验的 checkpoint。加载使用 `weights_only=True`，hash 绑定实际读取的同一批
checkpoint 字节；restricted unpickling 仍然不是 sandbox。

## 保留的模型与证据

[Model card](models/UniversalPhotographicStyleStateNet.model-card.md) 将 byte identity 与
历史私有测量明确分开。`.mlpackage` 作为研究证据保持字节不变，Rust runtime 不会选择它；
已经移除的 Swift 实验环境变量不是受支持 API。

```bash
python -m unittest discover -s Tests -p 'test_*.py' -v
```

回归覆盖 model shape、有限值序列化、byte identity、modality mask、source integrity、
数据集泄漏、calibration-only selection 和 publication failure。合成数据验证的是这些契约，
不是 native reconstruction accuracy。仍缺少真机 Photos import/edit/save/reopen 证据。

## 一手参考

- [PyTorch finite gradient checks](https://docs.pytorch.org/docs/stable/generated/torch.nn.utils.clip_grad_norm_.html)：拒绝非有限梯度，不把它们缩放成无效 checkpoint。
- [PyTorch checkpoint loading](https://docs.pytorch.org/docs/stable/generated/torch.load)：restricted loading 与 provenance 注意事项。
- [Core ML compute units](https://developer.apple.com/documentation/coreml/mlmodelconfiguration/computeunits)：允许使用的处理单元不能证明实际 ANE placement。
- [scikit-image 0.24 图像来源](https://scikit-image.org/docs/0.24.x/api/skimage.data.html)：对应 revision 的来源和 license 记录。

## 历史实验方法

[原生消费者与低维校准实验](methodology.md)保存冻结队列、响应/对照矩阵、训练负结果、ABI 失败与晋级门槛。`calibration.py` 重建冻结的共享 channel 校正，应用接口不接收 heldout 标签。旧缓存报告生成器与私有 Swift 产品入口不再作为维护入口。

# 摄影风格重建研究

[English](README.en.md) | 简体中文

本目录提供预测摄影风格资源的研究模型和工具。Rust 产品不会将这些模型用作转换实现。

预测的 `key1`、全局色调曲线（GTC）、光照图（light maps）和拍摄标量都是候选数据，不是恢复出的拍摄事实。离线误差、解析通过或不确定度阈值，不能证明原生渲染准确性，也不能证明 Apple Photos 支持可逆编辑。

## 实现位置

| 文件 | 职责 |
| --- | --- |
| [training.py](training.py) | 训练单图模型，以及带可选输入和可用性掩码的模型。 |
| [inference.py](inference.py) | 读取输入、核验文件身份，并生成数值有限的候选资源。 |
| [public_pretraining.py](public_pretraining.py) | 使用公开图像和解析生成的仿射标签进行训练；标签不来自 Apple 原生生成器。 |
| [calibration.py](calibration.py) | 拟合共享校正项，再应用冻结的系数；应用时不读取留出集标签。 |
| [apple_reverse_key1_training.py](../../xdremux_py/apple_reverse_key1_training.py) | 提供成对图像和单图自配对的共享训练操作。 |

这些文件不构成第二套产品转换器。[实验方法](methodology.md)记录历史队列、对照、失败尝试，以及重新开展实验的要求。

## 输入要求

主图是已应用方向信息的 RGB 图像，形状为 `3 × 256 × 256`。整数 RGB 使用 `[0, 255]` 范围，浮点 RGB 必须使用 `[0, 1]` 范围。程序根据数据类型确定范围。例如，最大字节值为 1 的图像仍按暗图处理。

模型从主图生成九个通道：三个 RGB 通道、亮度、Cb、Cr、对数亮度和两个梯度。

可选旁车文件提供额外的已解码输入：

| 旁车文件 | 数值要求 | 记录的证据 |
| --- | --- | --- |
| 线性 RGB | 已应用方向信息、位于 `[0, 1]` 范围的场景线性 RGB。 | 可用性掩码和 SHA-256。 |
| Gain Map | 正数线性曝光比，1 表示中性。 | 可用性掩码和 SHA-256。 |

DNG 内嵌预览是 RGB 图像，不是解码后的 RAW。标记字符串也不是解码后的 Gain Map。历史特征名 `has_raw` 和 `has_gain_map` 表示实际提供了对应的解码输入，文件扩展名和标记搜索不会设置这两个特征。保留的私有检查点并未学习每种可选输入的正例。

元数据使用独立掩码标记缺失值。布尔 EXIF 值不转换为数值拍摄测量。缺失的标量标签既不参与损失计算，也不更新任务不确定度；其评估指标为 `null`，不是零。

## 数据集与评估约束

通用数据集 v2 记录源清单、提取出的标签归档和每个缓存样本的身份。验证针对加载器实际解码的字节，同时检查数组形状、掩码和数值有限性。源文件身份重复、标签索引无效，或不同集合包含同一拍摄会话时，验证失败。

准备阶段在元数据提取前后核对源文件身份。旧版 v1 清单必须从已验证输入重新准备，不能未经原始字节核验就补齐缺失身份。

GTC 监督保留完整的 516 字节 Tag3 资源。该资源包含多种二进制字段，不能按平坦的 Float16 数组解释。预测结果长 516 字节，也不能证明其中的结构字段构成有效原生 GTC。

`base`、`multiscale_large` 和 `multimodal_large` 都是研究模型变体。热启动必须包含全部已学习参数，并保留目标语料的统计量。新增输入通道初始化为零。

`--resume` 只从同一清单对应的检查点继续加载权重，不恢复优化器或随机数生成器状态。所有运行，包括继续训练，都必须使用新的输出目录。通用模型训练器拒绝已有运行目录和输出符号链接。

预测、损失、梯度或选择指标出现非有限值时，训练停止。选择失败后不能复用旧 `best.pt`。训练器先在新的运行目录中写入临时文件，再发布检查点。拟合前固定清单身份，发布结果前重新核验。

校准集用于选择检查点，随后才评估留出集。`consumerProxyRMSE8` 使用逐像素残差平方，不使用逐图平均绝对误差的平方。编码 RGB 域的二次代理只用于训练正则化，不是 Apple 渲染器。

## 公开数据预训练

公开数据预训练至少需要三个独立源图像。拟合前固定训练集、校准集和留出集，三个集合不共享源图像。选定并冻结权重后，才评估留出集。

部分历史实验曾用名为「heldout」的集合选择检查点。这些数字属于选择集指标，不能作为独立留出集证据。

采集器记录发布者提供的许可元数据和署名信息，允许列表本身不能确定图像的法律状态。固定版本的 scikit-image 图像在解码前通过 Git blob 身份核验。

Commons 原文件的 SHA-1 与缩放后下载文件的 SHA-256 对应不同字节，不能按同一内容比较。下载得到的张量另有独立哈希。已有语料回执和张量不会被覆盖。

## 运行实验

从仓库根目录运行。先安装研究依赖：

```bash
python -m pip install '.[training]'
```

除非另行发布，原生样本清单、源媒体、缓存、检查点和原始逐图报告均属于私有输入。以下路径仅为示例；运行时使用已验证的输入和新的输出目录。

准备数据集并训练通用模型：

```bash
python -m research.oppo_styles.train prepare --native-manifest /data/native/manifest.json --output /data/universal-v2
python -m research.oppo_styles.train train --manifest /data/universal-v2/manifest.json --output /data/run
```

采集公开图像并使用合成标签训练：

```bash
python -m research.oppo_styles.pretrain collect --curated-only --output /data/public.json --image-directory /data/public-images
python -m research.oppo_styles.pretrain train --manifest /data/public.json --output /data/synthetic-run
```

只加载来源已核验的检查点。通用模型加载器使用 `weights_only=True`，并对实际加载的同一批字节计算哈希。受限反序列化不是沙箱。

生成候选归档：

```bash
python -m research.oppo_styles.predict --input /data/photo.heic --checkpoint /data/run/best.pt --output /data/candidate.zip
```

预测通过同一文件系统内的原子硬链接发布完整 ZIP，拒绝替换已有目标。准备阶段失败时不发布文件，也不修改输入。

ZIP 包含大小受限、数值有限的候选资源，以及与资源哈希绑定的报告，不是 HEIC 文件。报告记录源图和已解码旁车文件的身份，不编造硬件、人物、场景或拍摄字段。

## 解释验证结果

[UniversalPhotographicStyleStateNet 模型卡](models/UniversalPhotographicStyleStateNet.model-card.md)区分模型包身份与历史私有测量。保留的 `.mlpackage` 属于研究证据；已移除的 Swift 环境变量不是受支持 API。

运行 Python 回归测试发现命令：

```bash
python -m unittest discover -s tests -t . -p 'test_*.py' -v
```

测试覆盖模型形状、有限值序列化、字节身份、输入掩码、数据集完整性、集合泄漏、只由校准集选择模型，以及发布失败处理。合成测试验证这些软件约束，不证明原生重建精度。这些研究模型仍缺少真机 Photos 导入、编辑、保存和重新打开的证据。

[实验方法](methodology.md)保留训练负结果和私有 ABI 失败记录。旧缓存报告生成器与私有 Swift 产品入口不再作为维护入口。

## 一手参考

- [PyTorch 有限梯度检查](https://docs.pytorch.org/docs/stable/generated/torch.nn.utils.clip_grad_norm_.html)。
- [PyTorch 检查点加载](https://docs.pytorch.org/docs/stable/generated/torch.load)。
- [Core ML 计算单元](https://developer.apple.com/documentation/coreml/mlmodelconfiguration/computeunits)。
- [scikit-image 0.24 图像来源](https://scikit-image.org/docs/0.24.x/api/skimage.data.html)。

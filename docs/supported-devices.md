# 设备与文件兼容性

[English](supported-devices.en.md) | 简体中文

XDRemux 根据输入文件的结构和元数据选择转换方式，不以手机型号作为输入白名单。单个文件转换成功，不代表该型号的所有系统版本、相机模式和文件均受支持。

## 仓库样本覆盖范围

版本化样本为重复验证提供明确输入：

| 资产 | 已收录来源 | 样本能够证明的范围 |
| --- | --- | --- |
| ProXDR | OPPO Find X6 Pro 的 LHDR v1；Find X7 Ultra 的 LHDR v2，含 XPAN；Find X9 Ultra 的 UHDR，含高分辨率、大师和人像拍摄。 | 对应测试实际检查的格式和产品策略。 |
| Motion Photo | Samsung、Xiaomi、OPPO 和 vivo 的 JPEG、HEIC/HEIF 输入。 | 对应测试实际检查的解析、转换和验证行为。 |

[测试样本说明](../fixtures/README.md)介绍样本集，[SHA256SUMS](../fixtures/SHA256SUMS)标识具体文件。这些检查不能证明所有相机模式均受支持，也不能证明原厂相册或 Apple Photos 接受输出。

## 已有 ProXDR 拍摄型号记录

下表保留项目设备文档中已有的型号记录。仓库没有为每个条目分别提供样本或验证报告。因此，该表用于说明曾报告的拍摄格式范围，不是已验证的兼容性矩阵。

| 品牌或系列 | 机型 |
| --- | --- |
| 一加 | 一加 Ace2 Pro、一加 12、一加 Ace3、一加 Ace 3V、一加 Ace 3 Pro、一加 13、一加 Ace 5 系列、一加 13T、一加 Ace 6、一加 Ace 6T、一加 Turbo 6、一加 15、一加 15T、一加 Ace 5 至尊版 |
| OPPO K 系列 | K12、K12x、K13 Turbo 系列、K15 Pro 系列 |
| OPPO Find 系列 | Find X6、Find X6 Pro、Find N3、Find N3 Flip、Find X7、Find X7 Ultra、Find X8 系列、Find N5、Find X8s、Find X9 系列、Find N6 |
| OPPO Reno 系列 | Reno10 Pro、Reno10 Pro+、Reno11 Pro、Reno12 系列、Reno13 系列、Reno14 系列、Reno15 系列、Reno 16 系列 |
| realme GT 系列 | 真我 GT5 系列、真我 GT5 Pro、真我 GT6、真我 GT7 Pro、真我 GT7 Pro 竞速版、真我 GT7、真我 Neo7 Turbo、真我 GT8、真我 GT8 Pro |
| realme Neo 系列 | 真我 GT Neo6 SE、真我 GT Neo6、真我 Neo7、真我 Neo7 SE、真我 Neo7x、真我 Neo8 |
| realme 数字系列 | 真我 12 Pro、真我 12 Pro+、真我 13 Pro+、真我 13 Pro 至尊版、真我 13 Pro、真我 14 Pro+、真我 14 Pro、真我 14、真我 15、真我 15 Pro |

## Gain Map 差异

ProXDR 文件可能包含不同布局的 Gain Map。项目此前记录中，OPPO Find X8 Ultra、Find X9 系列和真我 GT8 Pro 理光模式可能使用 YCbCr 4:4:4 HDR Gain Map。其他文件可能使用 4:2:0 或单色 Gain Map。这些记录不能替代对实际输入的检查。

所选输出方式支持时，标准转换保留源 Gain Map 特征。`--oppo-compatible` 可能将其转换为兼容表示。不能只根据手机型号判断 Gain Map 布局。

## Motion Photo 要求

Motion Photo 支持取决于能否识别所需资源，以及对应格式行为是否经过测试，不由 ProXDR 型号记录控制。

输入必须包含照片资源、解析器能够定位的视频资源，以及转换所需的时间和容器信息。精确时间处理与遗漏资源说明见 [LibLivePhoto 接入契约](liblivephoto-integration.md)。

## Apple 人像要求

Apple 人像转换要求单张源照片包含兼容的人像资源。型号能够拍摄 ProXDR，不代表每张照片都包含景深、焦点、语义蒙版或恢复原图所需的数据。

完整模式与 OPPO 源数据模式的资源要求不同。选择模式前，参见 [Apple 功能文档](apple-features.md)。

## 报告不受支持的文件

除非明确决定公开，否则保留照片隐私。经过脱敏的容器诊断可能足以定位问题。

1. 保留原始文件。
2. 记录设备型号、系统版本和相机模式。
3. 记录准确的 XDRemux 错误信息。
4. 提供脱敏诊断，或已获授权的可复现样本。

新增兼容性结论需要可复现文件或测试支持。仅有型号名称不足以作为证据。

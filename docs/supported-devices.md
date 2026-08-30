# 支持设备

[English](supported-devices.en.md) | 简体中文

本文档列出已知能够拍摄 ProXDR HEIC 的设备型号。

型号在列表中，不代表每个系统版本、相机模式或单个文件都一定可以使用。XDRemux 会验证所选转换链路要求的文件结构和 metadata。

## 已知 ProXDR 拍摄机型

| 品牌或系列 | 机型 |
| --- | --- |
| 一加 | 一加 Ace2 Pro、一加 12、一加 Ace3、一加 Ace 3V、一加 Ace 3 Pro、一加 13、一加 Ace 5 系列、一加 13T、一加 Ace 6、一加 Ace 6T、一加 Turbo 6、一加 15、一加 15T、一加 Ace 5 至尊版 |
| OPPO K 系列 | K12、K12x、K13 Turbo 系列、K15 Pro 系列 |
| OPPO Find 系列 | Find X6、Find X6 Pro、Find N3、Find N3 Flip、Find X7、Find X7 Ultra、Find X8 系列、Find N5、Find X8s、Find X9 系列、Find N6 |
| OPPO Reno 系列 | Reno10 Pro、Reno10 Pro+、Reno11 Pro、Reno12 系列、Reno13 系列、Reno14 系列、Reno15 系列、Reno 16 系列 |
| realme GT 系列 | 真我 GT5 系列、真我 GT5 Pro、真我 GT6、真我 GT7 Pro、真我 GT7 Pro 竞速版、真我 GT7、真我 Neo7 Turbo、真我 GT8、真我 GT8 Pro |
| realme Neo 系列 | 真我 GT Neo6 SE、真我 GT Neo6、真我 Neo7、真我 Neo7 SE、真我 Neo7x、真我 Neo8 |
| realme 数字系列 | 真我 12 Pro、真我 12 Pro+、真我 13 Pro+、真我 13 Pro 至尊版、真我 13 Pro、真我 14 Pro+、真我 14 Pro、真我 14、真我 15、真我 15 Pro |

这个列表记录已知拍摄支持，不是代码 allow-list。

## Gain Map 差异

已知文件可能使用不同的 Gain Map 布局。

在已知实现中，OPPO Find X8 Ultra、Find X9 系列和真我 GT8 Pro 理光模式可能使用 YCbCr 4:4:4 HDR Gain Map。

其他文件可能使用 4:2:0 或单色 Gain Map。

标准转换链路会在所选输出链路支持时保留源 Gain Map 特征。`--oppo-compatible` 可能把表示降低为兼容形式。

不要只根据手机型号推断 Gain Map 布局。

## Motion Photo 支持

Motion Photo 采用能力检测和 fixture 验证，不由上方 ProXDR 型号表控制。

Motion Photo 输入必须包含静态资源、解析器可以确定的动态视频资源，以及所需的 timing/container 信息。

当前公开 fixture 集包含多种 Android Motion Photo 布局。见 [fixture 说明](../fixtures/README.md)。

## Apple 人像支持

Apple 人像转换要求单个源照片中存在兼容的人像资源。

设备型号支持 ProXDR，不代表每张照片都包含景深、焦点、语义或 restore-original 数据。

## 报告新文件

如果新设备或新系统生成的文件无法转换：

1. 保留原始文件。
2. 记录设备型号、系统版本和相机模式。
3. 提供准确的 XDRemux 错误信息。
4. 在足够定位问题时提供已去敏的容器诊断。
5. 除非你明确希望公开，否则不要发布包含个人内容的照片。

新增兼容性结论应该有可复现文件或测试支持。只有设备型号不足以作为证据。

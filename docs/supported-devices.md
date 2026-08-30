# 支持设备

[English](supported-devices.en.md) | 简体中文

本文档列出已知 ProXDR 拍摄设备族。它不保证每一个系统版本、相机模式或单个文件都可以使用全部转换功能。

XDRemux 会验证输入文件本身，而不是只检查设备型号。列表中的手机也可能生成不包含某条转换链路所需数据的照片。

## 已知 ProXDR 拍摄设备族

| 品牌或系列 | 已知型号或设备族 |
| --- | --- |
| OnePlus | Ace 2 Pro、12、Ace 3 系列、13 系列、Ace 5 系列、Ace 6 系列、Turbo 6、15 系列 |
| OPPO K | K12 系列、K13 Turbo 系列、K15 Pro 系列 |
| OPPO Find | Find X6 系列、Find N3 系列、Find X7 系列、Find X8 系列、Find N5、Find X9 系列、Find N6 |
| OPPO Reno | Reno10 Pro 系列以及项目样本和报告记录的后续 ProXDR Reno 机型 |
| realme GT | GT5 系列、GT6、GT7 系列、GT8 系列 |
| realme Neo | GT Neo6 系列、Neo7 系列、Neo8 |
| realme 数字系列 | 项目样本和报告记录的 12 到 15 系列 ProXDR 机型 |

转换器依赖文件结构，而不是营销型号，因此表格在适合时按系列归类。

## Gain Map 差异

已知文件可能使用不同的 Gain Map 布局。

部分较新的设备和拍摄模式会使用三通道 4:4:4 Gain Map。其他文件会使用 4:2:0 或单色 Gain Map。

标准转换链路会在输出链路支持时保留源通道特征。`--oppo-compatible` 可能把表示降低为兼容形式。

不要只根据手机型号推断 Gain Map 布局。

## Motion Photo 支持

Motion Photo 采用能力检测和 fixture 验证，不维护手机型号 allow-list。

Motion Photo 输入必须包含静态资源、可解析的动态视频资源，以及解析器可以确定的时间和容器数据。

当前公开 fixture 集包含多种 Android Motion Photo 布局。见 [fixture 说明](../fixtures/README.md)。

## Apple 人像支持

Apple 人像转换要求单个源照片中存在兼容的人像资源。

设备支持 ProXDR，不代表该设备拍摄的每张照片都有景深、焦点、语义或 restore-original 数据。

## 报告新文件

如果新设备或新系统生成的文件不能转换：

1. 保留原始文件。
2. 记录设备型号、系统版本和相机模式。
3. 提供准确的 XDRemux 错误信息。
4. 在足够定位问题时提供已去敏的容器诊断。
5. 除非你明确希望公开，否则不要发布包含个人内容的照片。

新增兼容性结论应该有可复现文件或测试支持，不能只依据型号名称。

# XDRemux

[English Version](README.en.md) | 中文版

XDRemux 可以将 OPPO、OnePlus、realme 设备拍摄的 ProXDR HEIC 照片转换为标准 HDR HEIC。

它会读取原始照片中的私有 HDR Gain Map 及元数据，并重新封装为符合 ISO 21496-1 标准的 HDR HEIC 文件。转换后的照片可以在 macOS、iOS、Android 等支持 HDR 照片显示的系统中查看。

## 什么时候需要这个工具？

如果你从 OPPO、OnePlus 或 realme 手机上拍摄了 ProXDR HEIC 照片，并希望它们在其他系统或软件里仍然以 HDR 方式显示，可以使用 XDRemux 转换。

## 输出能力

Swift CLI 提供三个可选开关。Apple 摄影风格与 Apple 人像是两个彼此独立、
默认关闭的能力，可以分别开启，也可以同时写入同一个最终 HEIC；
`--oppo-compatible` 与任一 Apple 输出互斥。所有开关都不指定时使用标准 ISO 默认模式。

| 模式 | 开关 | 结果 |
|---|---|---|
| 标准 ISO（默认） | 无 | 输出 ISO 21496-1 HDR；保留源 Base Image、原始通道结构和非 HDR 的 OPPO/QTI 元数据尾；源数据允许时 Gain Map 最高可达 HEVC RExt 4:4:4 |
| OPPO 相册兼容 | `--oppo-compatible` | 将 Gain Map 写成 OPPO 相册可消费的 HEVC Main Still Picture 4:2:0，并保留 OPPO 私有元数据尾 |
| Apple 摄影风格 | `--apple-photographic-styles` | 让照片在 Apple Photos 中使用摄影风格，并可切换风格或调整色调、色彩和强度；所需数据全部从当前照片生成 |
| Apple 人像 | `--apple-portrait` | 把 OPPO 人像照片转换成可在 Apple Photos 中继续调整景深和光圈的人像照片，并自动分析人物与头发等区域以改善虚化边缘 |

> [!IMPORTANT]
> 省略 `--output` 或 `--output-dir` 时会覆写输入文件。转换前请备份原片。

### 默认：标准 ISO HDR

```bash
# 单张
swift xdremux/swift-cli/XDRemux.swift convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic

# 批量
swift xdremux/swift-cli/XDRemux.swift batch \
  --input-dir photo_dump/ \
  --output-dir iso_output/
```

默认不启用 OPPO 专用兼容层。XDRemux 尽量保留原始 Base Image，只重建标准
ISO Gain Map 图；单通道源保持单通道，未被降采样的三通道源可保留最高
4:4:4/HEVC Range Extensions。已经是 4:2:0 的 Gain Map 不会被伪装成
4:4:4，因为丢失的色度信息无法恢复。

默认保留 OPPO/QTI/FileExtendedContainer 中的非 HDR 元数据，包括水印、
大师模式、拍摄参数、人像后期数据以及工具尚未识别的厂商字段；
`local.uhdr.*`、`local.hdr.*`、`src.local.hdr.*` 和 `hdr.*` 私有 HDR
条目会被物理移除，标准 ISO Gain Map 图仍是输出中生效的 HDR 显示图。

### `--oppo-compatible`：OPPO 相册兼容

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

此模式把高规格 Gain Map 转成 Main Still Picture 4:2:0，以触发 OPPO 相册的
HDR 显示。它仍保留 OPPO 私有元数据尾，因此适合需要回到 OPPO 生态的照片。

### `--apple-photographic-styles`：Apple 摄影风格

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

开启后，XDRemux 会根据当前照片的画面、亮度和色彩生成 Apple Photos 编辑摄影
风格所需的数据。输出继续保留 HDR，并可在 Photos 中切换风格，或调整色调、
色彩和强度。整个过程只使用正在转换的照片，不会借用其他照片的画面或编辑数据。

程序会根据照片中实际出现的人物、皮肤和天空等内容按需添加辅助区域；没有检测到
的内容不会用空蒙版凑数。面积很小但有效的区域仍会保留。如果当前 macOS 缺少
必要的系统分析能力，转换会明确报错，而不是生成内容不可靠的文件。

### `--apple-portrait`：转换 OPPO 人像景深

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic

# 批量时自动跳过没有完整人像资源的普通 HEIC
swift xdremux/swift-cli/XDRemux.swift batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

开启后，XDRemux 会读取 OPPO 人像照片原有的景深和光圈信息，并转换成 Apple
Photos 可以继续编辑的人像数据。程序会分析当前照片中的人物、皮肤、头发和其他
局部特征，也会在确认位置一致时利用 OPPO 自带的人物和头发蒙版改善边缘。不同
区域不会被随意混合，原有的对焦位置和模拟光圈也会尽量保留。

两个 Apple 开关可以同时启用，最终仍只生成一个 HEIC 文件，并同时保留 HDR、
摄影风格和人像编辑能力。批量转换时，普通非人像照片不会因为缺少景深数据而被
跳过；如果同时开启了摄影风格，它仍会输出可编辑摄影风格的照片。

Apple 输出面向 Apple Photos，`--oppo-compatible` 则面向 OPPO 相册，两者不能
同时启用；命令会在写文件前直接报错。不同机型和系统版本之间的人像虚化强度
可能仍有差异。

### Python CLI

> [!NOTE]
> 需要先安装依赖：`pip install pillow-heif Pillow numpy`

```bash
# 单张转换
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic

# 批量转换
python3 xdremux/python/XDRemux.py batch --input-dir photo_dump/

# OPPO 相册兼容输出（旧名 --oppo-compat 仍可用）
python3 xdremux/python/XDRemux.py convert --oppo-compatible --input IMG_001.heic
```

Apple 摄影风格和 Apple 人像转换目前由 Swift/macOS 实现；Python CLI 保持原有
HDR 转换能力。

### macOS App

源码位于：

```text
apps/macos/XDRemuxApp/
```

本地构建和运行：

```bash
scripts/build_and_run.sh run
```

## Swift CLI 输入处理模式

Swift CLI 支持 `--input-processing` 参数。普通用户通常不需要手动设置。

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic --input-processing hybrid
```

| 模式            | 说明                                                                                                           |
| ------------- | ------------------------------------------------------------------------------------------------------------ |
| `hybrid`      | 默认模式。保留原始 Base Image，只重新处理 HDR Gain Map。非 OPPO 输出保留原通道结构；开启 OPPO 兼容时，LHDR 使用已验证的 RGB-copy Gain Map。 |
| `system`      | 让系统 ImageIO 负责写出最终 HEIC。这个模式会重新编码 Base Image 和 Gain Map，适合用于对照系统行为。                                          |
| `passthrough` | 实验性模式。直接改写 HEIC 内部结构，用于验证和开发。普通用户不建议使用。                                                                      |

## 支持设备

XDRemux 适用于可以拍摄 ProXDR 照片的 OPPO、OnePlus、realme 设备。

在中国大陆销售且支持拍摄 ProXDR 照片的设备如下：

| 品牌/系列         | 机型名称                                                                                                                              |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 一加            | 一加 Ace2 Pro、一加 12、一加 Ace3、一加 Ace 3V、一加 Ace 3 Pro、一加 13、一加 Ace 5 系列、一加 13T、一加 Ace 6、一加 Ace 6T、一加 Turbo 6、一加 15、一加 15T、一加 Ace 5 至尊版 |
| OPPO K 系列     | K12、K12x、K13 Turbo 系列、K15 Pro 系列                                                                                                  |
| OPPO Find 系列  | Find X6、Find X6 Pro、Find N3、Find N3 Flip、Find X7、Find X7 Ultra、Find X8 系列、Find N5、Find X8s、Find X9 系列、Find N6                     |
| OPPO Reno 系列  | Reno10 Pro、Reno10 Pro+、Reno11 Pro、Reno12 系列、Reno13 系列、Reno14 系列、Reno15 系列、Reno 16 系列                                              |
| realme GT 系列  | 真我 GT5 系列、真我 GT5 Pro、真我 GT6、真我 GT7 Pro、真我 GT7 Pro 竞速版、真我 GT7、真我 Neo7 Turbo、真我 GT8、真我 GT8 Pro                                      |
| realme Neo 系列 | 真我 GT Neo6 SE、真我 GT Neo6、真我 Neo7、真我 Neo7 SE、真我 Neo7x、真我 Neo8                                                                      |
| realme 数字系列   | 真我 12 Pro、真我 12 Pro+、真我 13 Pro+、真我 13 Pro 至尊版、真我 13 Pro、真我 14 Pro+、真我 14 Pro、真我 14、真我 15、真我 15 Pro                                |

其中，OPPO Find X8 Ultra、Find X9 系列及真我 GT8 Pro（理光模式）在 Gain Map 实现中支持 **YCbCr 4:4:4 采样的 HDR Gain Map**。

## 仓库结构

| 路径                       | 用途                             |
| ------------------------ | ------------------------------ |
| `xdremux/swift-cli/`     | Swift CLI 主入口。                 |
| `xdremux/python/`        | Python CLI 与 HEIF I/O 辅助实现。    |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI App。             |
| `tests/`                 | 转换器测试。                         |
| `fixtures/`              | 小型测试样本与样本说明。                   |
| `scripts/`               | 本地构建、运行和验证脚本。                  |
| `experiments/`           | 实验性代码。                         |

## 已知限制

- 转换后的照片在 OPPO 相册中再次编辑并保存后，HDR Gain Map 及其 HDR 元数据可能会丢失。

本工具仅供技术研究使用。转换前请备份原始文件。作者不承担任何关于数据丢失的法律责任。

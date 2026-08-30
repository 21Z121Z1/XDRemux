# XDRemux

[English](README.en.md) | 简体中文

XDRemux 将厂商私有的 HDR 照片和 Android 实况照片转换为兼容标准 HDR 显示和 Apple Photos 的格式。

对于 ProXDR 照片，XDRemux 会读取原始 HDR Gain Map 及相关元数据，并将其重新封装为符合 ISO/TS 21496-1 的 HDR HEIC。

对于实况照片，XDRemux 会生成 Apple Live Photo 资源对，同时保留 HDR 静态照片、封面帧的呈现时间戳以及原始压缩视频和音频数据。

## 功能

| 功能 | 用法 | 输出 |
| --- | --- | --- |
| 标准 HDR | 默认 | ISO/TS 21496-1 HDR HEIC |
| 实况照片转换 | 自动识别 | Apple Live Photo HEIC + MOV |
| OPPO 相册兼容 | `--oppo-compatible` | 使用 4:2:0 Gain Map 的 HDR HEIC |
| Apple 摄影风格 | `--apple-photographic-styles` | 包含摄影风格编辑数据的 HEIC |
| Apple 人像 | `--apple-portrait` | 包含 Apple Photos 人像编辑数据的 HEIC |
| 照片分类 | `categorize` 或 `batch --categorize` | 按资产类型和拍摄模式分类 |

摄影风格和 Apple 人像使用 Apple 特有的编辑元数据。它们与标准 HDR 和 Live Photo 转换链路相互独立。

## 环境要求

Swift 版本需要：

- macOS 15 或更高版本
- Swift 6

克隆并构建项目：

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
```

正常使用建议构建 Release 版本：

```bash
swift build -c release
```

查看命令结构：

```bash
swift run xdremux --help
```

## HDR 转换

转换单张 ProXDR 照片：

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

转换整个目录：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

标准转换链路保留原始编码图像数据，并将 HDR Gain Map 元数据重建为 ISO/TS 21496-1 表示。

在输出格式支持的情况下，XDRemux 会保留源 Gain Map 的特征，包括单通道 Gain Map 和高精度三通道 Gain Map。

> [!IMPORTANT]
> 普通 HEIC 转换如果没有指定 `--output`，可能会替换输入文件。对于重要照片，请保留未经修改的原片。

## 实况照片转换为 Apple Live Photo

XDRemux 会自动识别支持的 Motion Photo，不需要额外参数。

```bash
swift run xdremux convert \
  --input IMG_001.jpg
```

转换结果是一组 Apple Live Photo 资源：

```text
IMG_001.heic
IMG_001.mov
```

两个文件共享同一个 Apple Live Photo 资产标识，必须作为一组保存。

转换会保留：

- HDR 静态照片数据，包括 Gain Map
- 源文件封面帧的呈现时间戳（PTS）
- 对应的 Apple `still-image-time`
- 压缩视频样本
- 压缩音频样本
- 支持的方向和几何元数据

视频和音频链路使用压缩样本透传。正常 Live Photo 重封装过程中，XDRemux 不会重新编码这些样本。

源 Motion Photo 不会被修改。

如果目标位置已经存在对应名称的 HEIC 或 MOV，XDRemux 会选择下一个可用文件名：

```text
IMG_001 (2).heic
IMG_001 (2).mov
```

如果显式指定 `--output`，XDRemux 不会静默覆盖已经存在的 HEIC 或配套 MOV。

批量转换同样会自动识别：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

同一个批次可以同时包含普通 HDR 照片和实况照片。XDRemux 会根据每个输入文件自动选择对应的转换链路。

## OPPO 相册兼容模式

部分 OPPO 相册版本不能正确处理高规格 Gain Map。

如果转换后的照片还需要在 OPPO 相册中查看或编辑，可以使用兼容模式：

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

该模式将 Gain Map 写为 HEVC Main Still Picture 4:2:0，并在存在相关数据时保留所需的私有元数据。

这个过程会降低 Gain Map 的色度表示精度，无法再恢复为原始的高精度表示。

## Apple 摄影风格

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux 可以生成 Apple Photos 摄影风格编辑界面所使用的元数据和图像资源。

这是 Apple 特有的兼容功能。具体实现和当前验证状态见 [Apple 功能](docs/apple-features.md)。

## Apple 人像

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

如果源照片包含所需的人像资源，XDRemux 可以将景深、焦点、光圈和语义数据转换为 Apple Photos 可以用于人像编辑的表示。

最终可以使用哪些编辑功能，取决于源照片实际保存的数据。

当前输入要求和验证边界见 [Apple 功能](docs/apple-features.md)。

## 照片分类

XDRemux 可以在不修改编码图像数据的情况下对照片进行分类：

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

使用 `--dry-run` 可以先查看计划生成的目录：

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/ \
  --dry-run
```

物理目录首先区分静态照片和 Live Photo，然后按主要拍摄模式分类。

通过验证的 Live Photo HEIC 和 MOV 始终作为同一个资产一起移动。

HDR、Gain Map、人像数据和厂商元数据等其他属性作为分类标签保留，不继续增加物理目录层级。

## Python CLI

Python 版本支持跨平台 HDR 转换、Motion Photo → Live Photo 转换和照片分类。

转换过程不依赖 Apple 平台框架。

安装：

```bash
pip install -e .
```

转换 HDR 照片：

```bash
xdremux-py convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

转换实况照片：

```bash
xdremux-py convert \
  --input IMG_001.jpg
```

Python Live Photo 链路同样保留 HDR 静态照片数据、封面时间信息和压缩媒体样本。

Apple 平台框架只用于 macOS 上的独立兼容性测试，不是 Python 转换器的运行时依赖。

也可以直接从仓库运行：

```bash
python3 -m xdremux_py --help
```

## macOS App

构建并运行 macOS 应用：

```bash
scripts/build_and_run.sh run
```

图形界面提供照片转换和分类功能。

## Swift Package

`XDRemuxCore` 和 `XDRemuxAppleFeatures` 可以作为 Swift Package 产品使用。

```swift
.package(
    url: "https://github.com/21Z121Z1/XDRemux.git",
    branch: "main"
)
```

标准转换链路使用 `XDRemuxCore`：

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)

let request = ConversionRequest(
    input: input,
    output: OutputTarget.file(outputURL).destination(for: input),
    configuration: ConversionConfiguration()
)

let result = try ConversionEngine.convert(request)
```

Apple 特有功能位于 `XDRemuxAppleFeatures`。

## 验证

XDRemux 使用自动化结构测试和真实文件集成测试。

Motion Photo 测试集包含多种 JPEG 和 HEIC/HEIF 文件结构。CI 会验证：

- 源文件完整性
- Motion Photo 资源边界
- 封面帧时间
- Live Photo 资产标识
- Apple `still-image-time` 元数据
- HDR Gain Map 保留
- 压缩视频样本一致性
- 压缩音频样本一致性
- Live Photo 文件结构
- macOS PhotoKit 兼容性

用于真机测试的 Live Photo 文件包也由同一套正式转换引擎生成。

## 文档

| 文档 | 内容 |
| --- | --- |
| [CLI 参考](docs/cli.md) | 命令、参数、默认值和退出行为 |
| [Apple 功能](docs/apple-features.md) | 摄影风格和 Apple 人像 |
| [支持设备](docs/supported-devices.md) | ProXDR 拍摄兼容性 |
| [开发文档](docs/development.md) | Package 结构和开发流程 |
| [技术实现](docs/xdremux/README.md) | HDR、HEIF、Gain Map 和容器实现 |

## 已知限制

- 不同应用对 ISO/TS 21496-1 HDR 的支持程度不同。
- 不同操作系统和图像查看器的 HDR 呈现可能不同。
- 转换后的 HDR 照片如果再次在厂商相册中编辑并保存，标准 HDR 元数据可能会被移除。
- Apple 特有编辑元数据的实际行为取决于读取它的 Apple Photos 版本。
- 设备本身受支持，不代表该设备拍摄的每张照片都包含转换所需的源数据。

如果源数据很重要，请保留原始照片。

## 许可证

见 [LICENSE](LICENSE)。

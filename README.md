# XDRemux

简体中文 | [English](README.en.md)

把 OPPO、OnePlus、realme 的 ProXDR 照片转成标准 HDR HEIC，让任何支持 ISO/TS 21496-1 的系统都能正常显示。

这些机型拍的 ProXDR 照片，HDR 信息藏在厂商私有数据块里 —— 换个相册看就是一张普通 SDR 照片。XDRemux 把私有 Gain Map 和元数据读出来，重新封装成 ISO/TS 21496-1 标准结构。主图像素数据逐字节保留，只重建 Gain Map 相关的容器结构。

## 快速开始

需要 macOS 15 或更高版本，以及 Swift 6。

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
```

转换单张：

```bash
.build/debug/xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

批量转换一个目录：

```bash
.build/debug/xdremux batch --input-dir photo_dump/ --output-dir converted/
```

> [!WARNING]
> `convert` 省略 `--output`、`batch` 省略 `--output-dir` 时会**直接覆盖原文件**。请先备份原片。

## 能输出什么

| 想要的结果 | 加什么参数 |
| --- | --- |
| 标准 ISO HDR | 默认，不加参数 |
| OPPO 相册也能识别的 HDR | `--oppo-compatible` |
| Apple Photos 摄影风格编辑 | `--apple-photographic-styles` |
| Apple Photos 人像景深编辑 | `--apple-portrait` |
| 按拍摄模式归档 | `categorize` 或 `batch --categorize` |

默认模式尽量保留原始主图和非 HDR 厂商元数据，只重建 Gain Map 结构。源文件是未降采样的 4:4:4 Gain Map 时保持 4:4:4；`--oppo-compatible` 会降为 OPPO 相册需要的 4:2:0，这一步不可逆。

全部参数、默认值和退出码见 [CLI 参考](docs/cli.md)，或直接跑 `swift run xdremux --help`。

## 按拍摄模式归档

读取 EXIF UserComment 里的拍摄模式，把照片复制进中文目录（`人像`、`夜景`、`大师模式` 等）。只复制，不修改也不删除源文件：

```bash
swift run xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run
```

`--dry-run` 只打印计划。给 `batch` 加 `--categorize` 可以让转换结果直接写进这些目录。Python 版本行为一致：

```bash
python3 xdremux/python/XDRemux.py categorize --input photo_dump/ --output-dir categorized/
```

读不出拍摄模式的照片会留在输出根目录，不算失败。

## Apple 摄影风格与人像

XDRemux 可以从照片自身生成 Apple Photos 需要的摄影风格和人像编辑数据，不读取任何 Apple donor 照片。人像功能要求源照片带完整的 OPPO 景深数据（`rear.depth`、`rear.depth.config`、`src.image`）。两个功能可以同时开启，写进同一个 HEIC；它们与 `--oppo-compatible` 互斥。

> [!IMPORTANT]
> 这两个功能**尚未通过正式验收**。当前可复现的证据只覆盖离线容器结构、ImageIO 和仓库 validator 检查，**没有**把"真机 Photos 导入、编辑、保存、退出、重开"作为通过项。哪些结论已经验证、哪些还没有，见 [Apple 功能文档](docs/apple-features.md)。

摄影风格求解是计算密集功能。批量处理请用 release 构建，比默认调试构建快数倍：

```bash
swift build -c release
.build/release/xdremux batch --apple-photographic-styles --input-dir photo_dump/ --output-dir styled/
```

## macOS App

```bash
scripts/build_and_run.sh run
```

图形界面覆盖转换和按模式分类两条流程，支持拖入文件或目录、预览、并发设置、断点续传和在访达中显示结果。

## Python CLI

跨平台，只做 HDR 转换，没有 Apple 功能。需要 Python 3.11 或更高版本。

```bash
pip install -r xdremux/python/requirements.txt
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic --output IMG_001_hdr.heic
```

## 作为 Swift Package 使用

```swift
.package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
```

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
let result = try ConversionEngine.convert(
    ConversionRequest(
        input: input,
        output: OutputTarget.file(outputURL).destination(for: input),
        configuration: ConversionConfiguration()
    )
)
```

Apple 功能在 `XDRemuxAppleFeatures` 产品里，入口是 `AppleFeatureConversionEngine`。详见[开发文档](docs/development.md)。

## 文档

| 文档 | 内容 |
| --- | --- |
| [CLI 参考](docs/cli.md) | 全部命令、参数、默认值、退出码 |
| [Apple 功能](docs/apple-features.md) | 摄影风格与人像的能力边界和验证状态 |
| [支持设备](docs/supported-devices.md) | 已知能拍 ProXDR 的机型 |
| [开发文档](docs/development.md) | 模块结构、Swift Package 集成、构建流程 |
| [技术实现](docs/xdremux/README.md) | HDR、HEIF 和 ISO 容器行为 |

## 已知限制

- 转换后的照片如果在 OPPO 相册里重新编辑并保存，标准 HDR Gain Map 可能会丢失。
- 不同应用对 HDR 峰值亮度、色彩管理和 Gain Map 的解释并不一致，同一张照片在不同地方看起来可能不同。
- 设备在支持列表里，不代表每张照片都带可转换的 Gain Map。实际结果取决于拍摄模式、固件版本和编辑历史。
- 本项目用于技术研究。转换结果不要作为原始照片的唯一副本。

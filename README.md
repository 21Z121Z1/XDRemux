# XDRemux

简体中文 | [English](README.en.md)

把 OPPO、OnePlus、realme 的 ProXDR 照片转成通用的 HDR 照片格式（ISO/TS 21496-1），让 iPhone、Mac 和其他支持这个标准的地方都能正常显示。

ProXDR 的 HDR 信息存在厂商私有数据里，换个相册打开就是一张亮部被压平的普通照片。XDRemux 把它翻译成通用格式，画面数据一个字节都不动。

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

默认只重写 HDR 那部分数据（业内叫 Gain Map，记录每个像素该提亮多少），画面、水印、大师模式这些一律原样保留。HDR 精度也保持原样；`--oppo-compatible` 必须把它降一档 OPPO 相册才认，降完回不去。

全部参数、默认值和退出码见 [CLI 参考](docs/cli.md)，或直接跑 `swift run xdremux --help`。

## 按拍摄模式归档

读出照片里记录的拍摄模式，把照片复制进对应的中文目录（`人像`、`夜景`、`大师模式` 等）。只复制，不修改也不删除源文件：

```bash
swift run xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run
```

`--dry-run` 只打印计划。给 `batch` 加 `--categorize` 可以让转换结果直接写进这些目录。Python 版本行为一致：

```bash
python3 xdremux/python/XDRemux.py categorize --input photo_dump/ --output-dir categorized/
```

读不出拍摄模式的照片会留在输出根目录，不算失败。

## Apple 摄影风格与人像

让转换后的照片在 Apple Photos 里支持摄影风格和人像景深编辑。这些数据都由你这张照片本身算出来，不用另找一张 iPhone 照片当模板。人像功能要求源照片用人像模式拍摄、景深数据还在 —— 后期编辑过的可能已经丢了。两个功能可以同时开，但都不能和 `--oppo-compatible` 一起用。

> [!IMPORTANT]
> 这两个功能**尚未通过正式验收**。检查只做到文件结构这一层：能打开、数据在。**没有**验证过导进真机 Photos 之后编辑、保存、再打开还撑不撑得住。详见 [Apple 功能文档](docs/apple-features.md)。

生成摄影风格数据很吃 CPU。批量处理请用 release 构建，比默认的调试构建快数倍：

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
pip install -e .
xdremux-py convert --input IMG_001.heic --output IMG_001_hdr.heic
```

不安装也可以，从仓库根目录用 `python3 xdremux/python/XDRemux.py` 或 `python3 -m xdremux_py` 调用同一套命令。

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
| [Apple 功能](docs/apple-features.md) | 摄影风格和人像能做什么、验证到了什么程度 |
| [支持设备](docs/supported-devices.md) | 已知能拍 ProXDR 的机型 |
| [开发文档](docs/development.md) | 模块结构、Swift Package 集成、构建流程 |
| [技术实现](docs/xdremux/README.md) | HDR、HEIF 文件结构和 ISO 标准的实现细节 |

## 已知限制

- 转换后的照片如果在 OPPO 相册里重新编辑并保存，HDR 数据可能会被抹掉。
- 各家应用对 HDR 亮度和色彩的处理方式不一样，同一张照片在不同地方看起来可能有差别。
- 设备在支持列表里，不代表它拍的每张照片都能转。实际取决于拍摄模式、固件版本和这张照片被编辑过没有。
- 本项目用于技术研究。转换结果不要作为原始照片的唯一副本。

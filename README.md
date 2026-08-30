# XDRemux

[English](README.en.md) | 简体中文

XDRemux 将厂商 HDR 照片转换为标准 HDR HEIC，并把支持的 Android Motion Photo 转换为 Apple Live Photo。

对于 ProXDR 输入，XDRemux 读取源 Gain Map 和相关 metadata，并写出采用 ISO/TS 21496-1 表示的 HDR HEIC。

对于 Motion Photo 输入，XDRemux 生成 Apple Live Photo HEIC + MOV 资源对。正常 Live Photo 链路保留 HDR 静态照片、封面帧呈现时间戳（PTS）以及压缩视频和音频样本。

## 主要功能

| 功能 | 选择方式 | 输出 |
| --- | --- | --- |
| 标准 HDR | 默认 | ISO/TS 21496-1 HDR HEIC |
| Motion Photo → Live Photo | 自动识别 | HEIC + MOV |
| OPPO 相册兼容 | `--oppo-compatible` | 兼容性 HDR HEIC |
| Apple 摄影风格 | `--apple-photographic-styles` | 包含 Apple 风格编辑资源的 HEIC |
| Apple 人像 | `--apple-portrait` | 包含 Apple 人像编辑资源的 HEIC |
| 分类 | `categorize` 或 `batch --categorize` | 按资产类型和拍摄模式分类 |

Apple 特有编辑功能有独立的支持和验证边界。见 [Apple 功能文档](docs/apple-features.md)。

## 环境要求

Swift package 需要 macOS 15 或更高版本，使用 Swift tools 6.0。

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
swift run xdremux --help
```

进行 CPU 开销较大的摄影风格工作时：

```bash
swift build -c release
```

Python package 需要 Python 3.11 或更高版本。

```bash
pip install -e .
xdremux-py --help
```

## 标准 HDR 转换

转换单张照片：

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

转换目录：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

所选转换链路允许时，标准链路会保留源压缩图像数据。转换器写入标准 Gain Map metadata，并保留输出链路支持的源 Gain Map 特征。

> [!IMPORTANT]
> 对普通 HDR 输入，如果没有 `--output`，目标可能就是输入文件。原文件重要时请保留未经修改的源文件。

## Motion Photo 转换为 Apple Live Photo

对于支持的 JPEG 和 HEIC/HEIF 输入，Motion Photo 会被自动识别。

```bash
swift run xdremux convert --input IMG_001.jpg
```

输出资源对使用同一个 basename：

```text
IMG_001.heic
IMG_001.mov
```

正常 Live Photo 链路保留：

- 源文件存在时的 HDR 静态照片和 Gain Map；
- 解析得到的源 cover-frame PTS；
- Apple `still-image-time`；
- 压缩视频样本；
- 源文件存在时的压缩音频样本；
- 支持的方向和几何 metadata。

源 Motion Photo 不会被修改。

隐式输出名称与已有 HEIC/HEIF 或配套 MOV 冲突时，XDRemux 会选择下一个可用 basename，例如 `IMG_001 (2)`。

显式设置 `--output` 时，XDRemux 拒绝覆盖已有 Live Photo 输出资源对。

batch 可以同时包含普通 HDR 照片和 Motion Photo：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

batch discovery、checkpoint 和 provenance 规则见 [CLI 参考](docs/cli.md)。

## OPPO 相册兼容

输出需要进入 OPPO 兼容链路时使用 `--oppo-compatible`：

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

这条链路可能为了兼容性降低 Gain Map 色度表示。已经丢弃的源色度无法在之后恢复。

## Apple 摄影风格

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

启用摄影风格时，当前默认 style-data producer 是 `constrained-solver`。

仓库还包含研究和诊断 producer，它们不是正常默认路径。

支持的组合和验收边界见 [Apple 功能](docs/apple-features.md)。

## Apple 人像

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

源照片必须包含转换器要求的人像资源。

当前资源和验证边界见 [Apple 功能](docs/apple-features.md)。

## 分类

只分类，不转换：

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

预览 Swift 计划：

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/ \
  --dry-run
```

通过验证的 Live Photo HEIC 和 MOV 在分类时始终作为一个资产一起移动。

batch 转换可以用 `--categorize` 对输出分类：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/ \
  --categorize
```

Python 提供相同的分类命令族：

```bash
python3 -m xdremux_py categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

## Python CLI

Python 支持跨平台 HDR 转换、Motion Photo → Live Photo 和分类。

```bash
xdremux-py convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic

xdremux-py convert --input IMG_001.jpg
```

Python 转换不依赖 Apple 平台 framework。适用时，macOS Apple framework 只用于独立兼容性测试。

Python 不生成摄影风格或 Apple 人像数据。

Swift 和 Python batch 的差异见 [CLI 参考](docs/cli.md)。

## macOS App

构建并运行 App：

```bash
scripts/build_and_run.sh run
```

App 直接链接 Swift package。

## Swift Package

公开 package product：

- `XDRemuxCore`
- `XDRemuxAppleFeatures`
- `xdremux`

示例：

```swift
.package(
    url: "https://github.com/21Z121Z1/XDRemux.git",
    branch: "main"
)
```

标准转换 API 使用 `XDRemuxCore`，Apple 特有转换引擎使用 `XDRemuxAppleFeatures`。

## 验证

仓库使用 unit test、仓库 policy test、真实 Motion Photo fixture、macOS framework 验证，以及在产品结论需要时使用真机验证。

公开 Motion Photo corpus 版本化保存在 `fixtures/`。strict gate 会检查准确 fixture identity，以及适用的 Live Photo timing、asset identity、Gain Map 保留、压缩样本透传和 publication safety 契约。

见[测试政策](docs/quality/testing.md)。

## 文档

| 文档 | 用途 |
| --- | --- |
| [文档索引](docs/README.md) | 全部当前和历史技术文档 |
| [CLI 参考](docs/cli.md) | 命令、默认值和输出规则 |
| [Apple 功能](docs/apple-features.md) | 摄影风格和 Apple 人像 |
| [支持设备](docs/supported-devices.md) | ProXDR 兼容性边界 |
| [开发文档](docs/development.md) | Package 结构和构建流程 |
| [测试政策](docs/quality/testing.md) | 必需验证证据 |
| [技术实现](docs/xdremux/README.md) | 当前实现契约 |
| [技术写作规范](docs/style-guide.md) | 文档术语和 STE 原则 |

## 已知限制

- HDR 呈现取决于操作系统和查看器。
- 厂商相册编辑并保存转换文件时可能移除标准 HDR metadata。
- Apple 特有编辑行为取决于 Apple Photos 和 framework 版本。
- 设备型号受支持，不代表每个文件都包含所有转换功能需要的数据。
- 结构验证不能替代依赖设备结论所需的 device evidence。

源数据重要时，请保留原始文件。

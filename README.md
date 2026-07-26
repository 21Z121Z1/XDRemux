# XDRemux

[English Version](README.en.md) | 中文版

XDRemux 将 OPPO、OnePlus 和 realme 设备拍摄的 ProXDR 照片转换为兼容性更好的 HDR HEIC。

它会读取照片中的私有 HDR Gain Map 和相关元数据，重新封装为符合 ISO/TS 21496-1 的 HDR HEIC。Swift 版本还可以选择生成适用于 Apple Photos 的摄影风格或人像编辑数据。

## 主要功能

| 模式 | 开关 | 用途 |
| --- | --- | --- |
| 标准 ISO HDR | 默认 | 转换为 ISO/TS 21496-1 HDR HEIC，适合跨平台查看 |
| OPPO 相册兼容 | `--oppo-compatible` | 生成更适合 OPPO 相册识别的 4:2:0 Gain Map |
| Apple 摄影风格 | `--apple-photographic-styles` | 让照片在 Apple Photos 中显示摄影风格编辑界面 |
| Apple 人像 | `--apple-portrait` | 将 OPPO 人像模式数据转换为 Apple Photos 支持的人像模式数据 |
| 拍摄模式分类 | `categorize` / `batch --categorize` | 按 UserComment 中的拍摄模式整理原片或转换结果 |

Apple 摄影风格和 Apple 人像可以同时启用，并写入同一个 HEIC。Apple 相关的输出选项与 `--oppo-compatible` 不能同时使用。

> [!NOTE]
> Apple 摄影风格和 Apple 人像目前属于实验性兼容功能。不同照片、设备型号和系统版本的结果可能存在差异。

## 运行要求

Swift 版本需要：

- macOS 15 或更高版本
- Swift 6
- 支持的 OPPO、OnePlus、realme 设备拍摄的 ProXDR HEIC 照片

克隆仓库后进入项目目录：

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
```

查看完整命令行帮助：

```bash
swift run xdremux --help
```

> [!IMPORTANT]
> 单张转换省略 `--output` 时会覆写输入文件。批量转换省略 `--output-dir` 时会把结果写入输入目录。请先备份原片。

## 标准 ISO HDR

不添加功能开关时，XDRemux 使用标准 ISO HDR 模式。

单张转换：

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic
```

批量转换：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir iso_output/
```

此模式会尽量保留原始 Base Image 和非 HDR 厂商元数据，只重建标准 Gain Map 结构。

源 Gain Map 为单通道时继续保持单通道；源文件包含未降采样的三通道 4:4:4 Gain Map 时，可以保留其原始通道结构。

## OPPO 相册兼容模式

```bash
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

此模式会将高规格 Gain Map 转换为 HEVC Main Still Picture 4:2:0，并尽可能保留 OPPO 相册可能需要的私有元数据。

该模式适合需要将照片重新导入 OPPO 相册的情况。它不适合与 Apple 摄影风格或 Apple 人像同时使用。

## Apple 摄影风格

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux 会根据当前照片的画面、亮度、色彩和语义区域生成摄影风格所需的数据。

输出照片可在 Apple Photos 中支持：

- 切换摄影风格
- 调整色调 Tone
- 调整色彩 Color
- 调整风格强度

生成过程会把照片在编辑器中的 Tone/Color 联动响应与原生 iPhone 样本包络对齐：当检测到超出包络的响应时，求解会一并修正，并保证结果不劣于修正前；本就合规的照片只做一次快速验证，几乎不增加耗时。

> [!TIP]
> 摄影风格求解是计算密集功能。批量处理建议使用 release 构建：先 `swift build -c release`，再运行 `.build/release/xdremux`，速度可比默认调试构建快数倍。

## Apple 人像模式

单张转换：

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic
```

批量转换：

```bash
swift run xdremux batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

Apple 人像模式的转换要求输入照片包含一组完整且相互匹配的 OPPO 人像模式信息：

- ISO/TS 21496-1 Gain Map
- `rear.depth`
- `rear.depth.config`
- 完整的 `src.image`

XDRemux 会转换原有景深、焦点和模拟光圈信息，并分析人物、皮肤、头发等区域以改善虚化边缘。能够保留的编辑能力取决于源文件实际包含的数据。

Apple 人像输入的 `src.image` Gain Map 必须能由 macOS ImageIO 正确读取。目前支持 RGB 4:4:4 和灰度 Gain Map；缺失、损坏或不符合要求的 Gain Map 会使转换直接失败。

### JPEG 人像输入

部分 OPPO 人像以 JPEG 作为外层容器。JPEG 输入仅在启用 `--apple-portrait` 时接受，最终输出仍会被转换为 HEIC。

批量处理 JPEG 人像时需要显式指定匹配规则：

```bash
swift run xdremux batch \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

没有启用 Apple 人像时，标准 ISO、OPPO 相册兼容和单独的摄影风格模式仍只接受 HEIC 输入。

## 同时写入摄影风格和人像

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

最终只会生成一个 HEIC，并同时尝试保留：

- HDR 显示
- Apple 摄影风格编辑
- Apple 人像景深编辑

批量转换时，如果某张普通照片没有完整人像数据，但启用了摄影风格，XDRemux 仍可为该照片生成摄影风格输出。

## 批量转换

默认批量行为包括：

- 同时使用最多 4 个并发任务
- 自动记录转换进度
- 中断后继续处理
- 跳过已经存在且验证有效的输出
- 失败的文件在下次运行时重新尝试

常用示例：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/ \
  --glob '*.heic' \
  --jobs 4
```

更多断点续传、覆盖和诊断参数请查看：

```bash
swift run xdremux --help
```

## 按拍摄模式分类

独立分类会递归扫描 HEIC、HEIF 和 JPEG，只复制照片，不修改或删除源文件：

```bash
swift run xdremux categorize \
  --input photo_dump/ \
  --input another_photo.heic \
  --output-dir categorized/ \
  --jobs 4
```

使用 `--dry-run` 可以只查看规划结果。分类目录使用固定中文名称：普通拍照、大师模式、RICOH GR、专业模式、人像、夜景、全景、延时摄影、超清、证件照、贴纸、超级文本、合影、双重曝光和美颜。

缺少 UserComment、格式错误、读取失败，或只有未知标记且无法确定主模式的照片会保留在目标根目录。格式错误或读取失败仍会复制文件，但命令返回非零；缺少 UserComment 或未知主模式不视为错误。普通拍照以及只有 HDR、滤镜、水印等已知附加标记的照片会进入 `普通拍照/`。同名同内容文件会跳过；同名但内容不同的文件会稳定命名为 `文件名 (2).heic` 等。

批量转换时可直接按拍摄模式写入转换结果：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/ \
  --categorize
```

`convert` 不接受 `--categorize`；该开关只用于 `batch`。

分类只读取本地文件中的 EXIF/UserComment，不声明或模拟 OPPO 相册在设备端的识别行为。

## 验证输出

验证 Apple 摄影风格或组合输出：

```bash
swift run xdremux validate-apple \
  --input IMG_001_apple.heic \
  --expect-portrait \
  --json IMG_001_apple.validation.json
```

只验证 Apple 人像结构：

```bash
swift run xdremux validate-portrait \
  --input IMG_001_apple_portrait.heic \
  --json IMG_001_apple_portrait.validation.json
```

验证器会检查 HEIC auxiliary image、Focus XMP 和相关元数据结构。

Apple 人像转换可能还会在输出旁生成 `*.portrait-manifest.json`，用于记录输入资源、转换结果和兼容性诊断。该 JSON 文件不需要随照片导入 Apple Photos。

离线验证只能证明文件结构符合当前检查规则，不能替代 Apple Photos 中的导入、重新对焦、保存和重新打开测试。

## Python CLI

Python CLI 提供标准 HDR、OPPO 相册兼容转换和与 Swift 一致的拍摄模式分类，不包含 Apple 摄影风格和 Apple 人像功能。

安装依赖：

```bash
pip install pillow-heif Pillow numpy
```

单张转换：

```bash
python3 xdremux/python/XDRemux.py convert \
  --input IMG_001.heic
```

批量转换：

```bash
python3 xdremux/python/XDRemux.py batch \
  --input-dir photo_dump/
```

独立分类与分类输出：

```bash
python3 xdremux/python/XDRemux.py categorize \
  --input photo_dump/ \
  --input another_photo.heic \
  --output-dir categorized/ \
  --dry-run

python3 xdremux/python/XDRemux.py batch \
  --input-dir photo_dump/ \
  --output-dir converted/ \
  --categorize
```

OPPO 相册兼容输出：

```bash
python3 xdremux/python/XDRemux.py convert \
  --oppo-compatible \
  --input IMG_001.heic
```

## macOS App

macOS App 源码位于：

```text
apps/macos/XDRemuxApp/
```

本地构建并运行：

```bash
scripts/build_and_run.sh run
```

App 顶部可在“转换”和“按模式分类”之间切换。分类页支持一次添加多个文件或目录、扫描预览模式数量和目标路径、复制、取消，以及在 Finder 中显示结果；不选择统一目标目录时，每张照片以自身所在目录为分类根。转换设置中的“按拍摄模式分类输出”开关会把转换结果写入相同的中文模式目录。

## 作为 Swift Package 使用

其他 SwiftPM 项目可以直接依赖本仓库：

```swift
dependencies: [
    .package(
        url: "https://github.com/21Z121Z1/XDRemux.git",
        branch: "main"
    )
]
```

基础 HDR 转换使用 `XDRemuxCore`：

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

需要 Apple 摄影风格或 Apple 人像时，添加 `XDRemuxAppleFeatures` 产品并使用 `AppleFeatureConversionEngine`。

## 开发

构建：

```bash
swift build
```

运行测试：

```bash
swift test
```

主要目录：

| 路径 | 用途 |
| --- | --- |
| `Sources/XDRemuxCore/` | HDR、HEIF、元数据和批量转换核心 |
| `Sources/XDRemuxAppleFeatures/` | Apple 摄影风格和人像功能 |
| `Sources/XDRemuxCLI/` | Swift 命令行入口 |
| `xdremux/python/` | Python CLI |
| `apps/macos/XDRemuxApp/` | macOS App |
| `Tests/` | 自动化测试 |
| `scripts/` | 构建和验证脚本 |

## 设备与文件兼容性

XDRemux 面向能够拍摄 ProXDR 照片的 OPPO、OnePlus 和 realme 设备。

项目不依赖固定的机型白名单，而是检查输入文件中实际存在的 Gain Map 和厂商元数据。相同机型在不同固件版本下也可能生成不同结构；不符合当前输入要求的文件会明确报错。

并非所有系统相册或第三方软件都支持 ISO/TS 21496-1、4:4:4 Gain Map、Apple 摄影风格或 Apple 人像数据。

## 已知限制

- 在 OPPO 相册中重新编辑并保存转换后的照片，可能导致标准 HDR Gain Map 或 HDR 元数据丢失。
- 转换过程可能覆写输入文件，请保留未经修改的原片。
- 不同应用对 HDR 峰值亮度、色彩管理和 Gain Map 的解释可能不同。

本项目用于技术研究。转换结果不应作为原始照片的唯一副本。

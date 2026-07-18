# XDRemux

[English Version](README.en.md) | 中文版

XDRemux 可以将 OPPO、OnePlus、realme 设备拍摄的 ProXDR HEIC 照片转换为标准 HDR HEIC。

它会读取原始照片中的私有 HDR Gain Map 及元数据，并重新封装为符合 ISO 21496-1 标准的 HDR HEIC 文件。转换后的照片可以在 macOS、iOS、Android 等支持 HDR 照片显示的系统中查看。

## 什么时候需要这个工具？

如果你从 OPPO、OnePlus 或 realme 手机上拍摄了 ProXDR HEIC 照片，并希望它们在其他系统或软件里仍然以 HDR 方式显示，可以使用 XDRemux 转换。

## 使用方式

仓库根目录现在是正式的 Swift Package。需要 macOS 15 或更高版本以及 Swift 6，
克隆仓库后可直接运行：

```bash
swift run xdremux --help
swift run xdremux convert --input IMG_001.heic --output IMG_001_iso.heic
```

旧的单文件命令仍作为兼容入口保留，参数、输出和退出码与正式入口相同：

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

开发和验证使用 `swift build` 与 `swift test`。可复用库产品为
`XDRemuxCore` 和 `XDRemuxAppleFeatures`；正式命令行为 `xdremux`，内部验证工具为
`xdremux-dev`。

Apple 摄影风格和人像路径使用三个由 SwiftPM 构建的隔离 helper。首次从源码使用
Apple 功能前先运行一次 `swift build`（不要限定单一 product），以同时生成这些
helper；它们不会在运行时编译，也不会以源码形式进入 App bundle。

### 正式 CLI

| 范围 | 参数 |
| --- | --- |
| 单文件 | `convert --input <文件> [--output <文件>]` |
| 批量 | `batch --input-dir <目录> [--output-dir <目录>] [--glob <模式>] [--jobs <数量>]` |
| 写入 | `--overwrite`、`--discard-portrait-data` |
| 产品模式 | `--oppo-compatible`、`--apple-photographic-styles`、`--apple-portrait` |
| 输出 | `--quiet`、`--verbose`、`--debug`、`--format text|json|jsonl` |
| 语言 | `--language auto|zh-Hans|en` |

Apple 摄影风格和 Apple 人像可以组合；`--oppo-compatible` 与任一 Apple 模式
互斥，并在参数解析阶段报错。`--quiet` 只保留错误和最终结果，`--verbose` 增加
逐文件结果，`--debug` 再增加内部配置、临时路径和完整诊断链。

文本帮助写入 stdout；人类可读的进度、警告和错误写入 stderr。交互式终端使用
原地进度，重定向、管道和 CI 自动退化为无 ANSI 的逐行输出。`json` 和 `jsonl`
写入 stdout，字段名、事件名和错误代码始终为英文，不随显示语言改变：

```json
{"schema_version":1,"event":"conversion_failed","error_code":"source_gain_map_missing","input":"IMG_001.heic","message":"输入照片没有可用的 HDR Gain Map。"}
```

语言选择顺序为 `--language`、`XDREMUX_LANGUAGE`、系统首选语言，最后回退英文。
退出码为：`0` 成功、`1` 内部错误、`2` 参数错误、`3` 输入错误、`4` 输出或
Apple runtime 错误、`5` 批量部分失败、`130` 被 Ctrl+C 中断。

批量输出保留输入目录的相对结构。重跑时，已有且通过轻量验证的输出会被跳过；
无效输出会重新转换。每个文件通过同目录临时文件原子安装，单文件失败不会终止
整个批次，失败项写入 `<output-dir>/xdremux-failures.json`。批量状态不再依赖
checkpoint、配置哈希或 mtime journal。

### 开发者 CLI

实验参数和 validator 只在 `xdremux-dev` 中提供，普通用户无需使用：

```bash
swift run xdremux-dev convert \
  --input IMG_001.heic \
  --family x7 \
  --input-processing hybrid \
  --diagnostics-dir diagnostics/

swift run xdremux-dev validate-portrait --input IMG_001_apple_portrait.heic
```

开发者入口还保留 `--oppo-compat`、`--oppo-camera-tail`、`--tmap-format`，以及
`validate-apple`、`validate-portrait`、`portrait-self-test`。这些参数不会出现在
正式 `xdremux --help` 中。

### 作为 Swift Package 集成

其他 SwiftPM 项目可以直接依赖 GitHub 仓库：

```swift
dependencies: [
    .package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
]
```

目标按需要选择 `.product(name: "XDRemuxCore", package: "XDRemux")` 或
`.product(name: "XDRemuxAppleFeatures", package: "XDRemux")`。基础转换入口为：

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

需要 Apple 摄影风格或人像时，设置 `AppleFeatureOptions` 对应配置并调用
`AppleFeatureConversionEngine.convert(_:)`。发布第一个稳定 tag 后，外部依赖应改用
语义版本范围，而不是长期跟随 `main`。

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
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic

# 批量
swift run xdremux batch \
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
swift run xdremux convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

此模式把高规格 Gain Map 转成 Main Still Picture 4:2:0，以触发 OPPO 相册的
HDR 显示。它仍保留 OPPO 私有元数据尾，因此适合需要回到 OPPO 生态的照片。

### `--apple-photographic-styles`：Apple 摄影风格

```bash
swift run xdremux convert \
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
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic

# 单个失败不会终止批次；失败项写入 xdremux-failures.json
swift run xdremux batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/

# OPPO 标准 HDR JPEG 人像（原目录生成同名 .heic，保留 .jpg 原片）
swift run xdremux batch \
  --apple-photographic-styles \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/
```

开启后，XDRemux 会读取 OPPO 人像照片原有的景深和光圈信息，并转换成 Apple
Photos 可以继续编辑的人像数据。程序会分析当前照片中的人物、皮肤、头发和其他
局部特征，也会在确认位置一致时利用 OPPO 自带的人物和头发蒙版改善边缘。不同
区域不会被随意混合，原有的对焦位置和模拟光圈也会尽量保留。

当前生产路径会强类型解析 `rear.depth.config` v1–v4、`rear.depth` 的量化范围、
near-object/语义状态和 22 点光圈曲线，并按固件公式把 rank 还原到
`CalFocusDepthEngine` 使用的 float-depth 域。焦点分支严格由 near-object、
`sceneClass` 和 `focusRoiType` 调度；无保存 landmark 的 PetScene 已按固件使用全图
隔像素 20-bin、2% histogram。manifest 分别记录 `branchEvidence`、`roiEvidence` 和
`statisticEvidence`，其余 face/PetFace/near-object helper 仍明确标为 fallback。Apple renderer 只保留按物理镜头验证不变的
153 条静态 `REND` records；`0x0190...0x0199` 与
`0x01c2...0x01c5` 每张照片重新生成，不再保存或复制完整 donor `REND`。输出旁会
生成 `*.portrait-manifest.json`，逐项记录焦点分支、OPPO-domain disparity、Apple
relative disparity、镜头 profile、动态 records、证据等级和所有 fallback。
iOS 26.5 producer 的三个输入已闭环为 `ISOSpeedRating`、`ExposureTime` 和
`GainMapHeadroom`；其中 `0x01c5` 必须与当前输出 Gain Map 的 headroom 相同。
`ControlLogicForXHLRB` 的 32 字节输出布局和最终 CPU scaler 已按固件实现，
validator 会反算并检查这些关系。固件内置的默认 exposure/clipped smoothstep、blur/
intensity 阈值，以及默认 Simple Lens Model ROI、shift dead zone 和 disparity scale 已
直接恢复；但实际 `RenderingV<version><suffix>` 每镜头 override 与 clipped-pixel 定义
仍未取得。因此受控样本给出的非零 tuning 上限和进入 scaler 前的 activation 继续标记为
`controlled_corpus_fit`。iOS ObjC wrapper 在 macOS 不存在；已确认其 Metal kernel
本身可由当前 macOS/M1 Pro 创建，但生产转换不静默依赖外置 IPSW 文件。

可用独立验证命令检查 ImageIO auxiliary、Focus XMP、`REND` round-trip/动态关系
和 donor 污染：

```bash
swift run xdremux-dev validate-portrait \
  --input IMG_001_apple_portrait.heic \
  --json IMG_001_apple_portrait.validation.json
```

Apple 人像桥接也支持带 ISO/TS 21496-1 Gain Map、`rear.depth`、
`rear.depth.config` 和 `src.image` 的 OPPO HDR JPEG。三通道 4:4:4 Gain Map
JPEG 直接取自 `src.image`，并与其中未虚化的 Base JPEG 以相同存储方向组成
Ultra HDR 中间件后再转换为 HEIC；外层标准 Gain Map 提供 ISO 参数。当 ImageIO
用一个共享参数项描述三路相同曲线时，XDRemux 会把该参数等值映射到三个通道。
此 JPEG 桥接需要 `ultrahdr_app` 位于 `PATH`。批处理时需用
`--glob '*.jpg'` 显式选择 JPEG 人像。JPEG 输入仅在启用
`--apple-portrait` 时接受；可以同时启用摄影风格，但默认 ISO、单独摄影风格和
OPPO 相册兼容模式仍只接受原有 HEIC 输入。

两个 Apple 开关可以同时启用，最终仍只生成一个 HEIC 文件，并同时保留 HDR、
摄影风格和人像编辑能力。批量转换时，普通非人像照片不会因为缺少景深数据而被
跳过；如果同时开启了摄影风格，它仍会输出可编辑摄影风格的照片。

Apple 输出面向 Apple Photos，`--oppo-compatible` 则面向 OPPO 相册，两者不能
同时启用；命令会在写文件前直接报错。不同机型和系统版本之间的人像虚化强度
可能仍有差异。离线验证通过不等同于 Photos 的保存重开/重新对焦实机验收。
载荷移植会把第一阶段 base/gain 项所关联的原始 `hvcC` 属性一并移入人像
scaffold，并同步调整 `meta`/`iprp`/`ipco`、`iloc` 和 `mdat`；因此 111/112 字节
`hvcC` 差异的 230 mm 原片不再被错误拒绝，也不会把原始 HEVC 载荷挂在不匹配的
codec graph 下。主图 EXIF 仍为 230 mm，Apple 辅助校准饱和到已验证的 120 mm
物理 profile，不会虚构 Apple 10× 镜头。

2026-07-17 的单样本 macOS Photos 验证已确认 139 mm/3× crop 组合输出的原始
f/6.3、f/1.4、f/16、重新选择焦点，以及保存重开后的 Portrait 与 Photographic
Styles 能力。这个结果不代表多 profile 矩阵或 iOS 实机已经通过。

同日的 230 mm/5× 饱和样本也通过 macOS Photos：原始 f/10、f/1.4、f/16、
背景/主体重新选择焦点，以及保存、离开照片、重新打开后的 f/1.4 人像状态均
保留。Photos 仍显示其低分辨率/不支持格式辅助徽章，因此这同样不是跨系统或
完整矩阵验收。

上述 Photos 结果来自较早构建。本次 `producer-fallbacks` 候选已通过离线验证，
但 UI 自动化连续遇到 ScreenCaptureKit `-3811`，未能导入并完成最新构建的保存
重开矩阵；因此没有把旧结果记作本次结果。

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
scripts/build_and_run.sh build
scripts/build_and_run.sh run
scripts/build_and_run.sh verify
```

`build` 只构建，`run` 构建并启动，`verify` 还会检查 bundle、helper 签名和进程；
`debug` 使用 LLDB，`logs` 只显示 `com.proxdr.XDRemuxApp` subsystem，
`logs --all` 才显示该进程的完整日志，`clean` 只删除 XDRemux 自己的 DerivedData。
默认构建日志保持简短，完整 `build.log` 与 `.xcresult` 保存在 DerivedData 中；
`--verbose` 会实时显示完整 `xcodebuild` 输出。

App 直接链接 `XDRemuxCore` 和 `XDRemuxAppleFeatures`，用同一个
`ConversionRequest`、结构化事件和取消令牌驱动 ViewModel，不启动或解析完整 CLI。
Vision、VideoToolbox 和 Neutrino 私有流程仍在 `Contents/Helpers` 的独立签名进程中。

## Swift CLI 开发者输入处理模式

`--input-processing` 是 `xdremux-dev` 的实验参数，不属于正式用户 CLI。

```bash
swift run xdremux-dev convert --input IMG_001.heic --input-processing hybrid
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

| 路径 | 用途 |
| --- | --- |
| `Package.swift` | 根目录 SwiftPM 清单，声明两个库产品以及 `xdremux`、`xdremux-dev` 可执行产品。 |
| `Sources/XDRemuxCore/` | 与 CLI 和 UI 无关的转换模型、HDR、HEIF、Metadata 与 Batch 核心。 |
| `Sources/XDRemuxAppleFeatures/` | Apple 语义场景、摄影风格和人像功能。 |
| `Sources/XDRemuxCLI/` | 命令分派、共享参数解析、本地化资源和终端/JSON 输出。 |
| `Sources/XDRemuxExecutable/` | 正式 `xdremux` 的轻量入口。 |
| `Sources/XDRemuxDevExecutable/` | 内部参数和 validator 的 `xdremux-dev` 入口。 |
| `xdremux/swift-cli/` | 旧 `swift <file>` 命令的兼容转发入口。 |
| `xdremux/python/` | Python CLI 与 HEIF I/O 辅助实现。 |
| `apps/macos/XDRemuxApp/` | 依赖 Swift Package 的 macOS SwiftUI App shell。 |
| `Tests/` | SwiftPM 单元测试、Python 回归测试与验证 harness。 |
| `fixtures/` | 小型测试样本与样本说明。 |
| `scripts/` | 本地构建、运行和验证脚本。 |
| `experiments/` | 实验性代码。 |

## 已知限制

- 转换后的照片在 OPPO 相册中再次编辑并保存后，HDR Gain Map 及其 HDR 元数据可能会丢失。

本工具仅供技术研究使用。转换前请备份原始文件。作者不承担任何关于数据丢失的法律责任。

# XDRemux CLI 参考

[English](cli.en.md) | 简体中文

XDRemux 的产品入口是 Rust `xdremux` CLI。输入类型、Motion Photo / ProXDR 路由以及 HDR / Gain Map 源结构都由程序自动识别；标准转换不要求用户选择格式、设备代际或底层处理策略。

完整 Mac 产品交付面向 Apple silicon（`arm64`）和 macOS 27。底层 Rust 核心保留 Linux、Windows、macOS 构建及可移植测试。核心构建或较旧的编译下限不证明 Styles SPI 或其他 Apple 原生功能可用。这些功能仍需文档规定的 macOS 27 SDK、运行时 API、硬件及独立原生验收检查。

Swift package 只作为 Apple 平台 capability adapter，剩余 Python package 只作为研究/训练工具。它们都不是第二套 XDRemux 实现，不再定义新的 CLI 产品语义或产品 policy。

## 命令

| 命令 | 功能 |
| --- | --- |
| `convert` | 转换一张 ProXDR 照片；支持的 Motion Photo 会自动转换为 Live Photo。 |
| `batch` | 批量发现并转换支持的照片资产。 |
| `categorize` | 不转换，只按资产类型和主要拍摄模式分类。 |
| `inspect` | 检查输入类型和关键结构。 |
| `validate` | 验证 ISO HDR HEIF 或 Live Photo 输出。 |

运行帮助：

```bash
xdremux --help
xdremux convert --help
```

## 终端语言

使用 `xdremux --lang en --help` 或 `xdremux --lang zh-CN --help` 选择英文或
简体中文终端显示。全局选项也可放在子命令后。选择顺序为 `--lang`、
`XDREMUX_LANG`、`LC_ALL`、`LC_MESSAGES`、`LANG`，最后默认英文。忽略空环境变量。
`zh_CN.UTF-8` 等受支持的 `zh` locale 选择简体中文；`C` 和 `POSIX` 选择英文。
不支持的环境 locale 回退英文；显式指定不支持的 `--lang` 则返回用法错误。

语言不改变 JSON 文档、命令/选项名称、枚举值、退出码或输入路径参数。
翻译范围为人类可读帮助、标签和 CLI 持有的错误上下文。Clap 详细参数诊断和
底层库/原生错误仍可能使用英文。

## `convert`

标准转换不需要模式参数：

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

对于普通 ProXDR，如果省略 `--output`，输出目标就是输入路径，并使用原子发布替换文件。

对于支持的 Motion Photo：

```bash
xdremux convert --input IMG_001.jpg
```

XDRemux 会自动识别 Motion Photo，并发布匹配的 HEIC + MOV Live Photo pair。Motion Photo 不做原位转换；未指定输出时会选择不会与现有 HEIC/MOV 冲突的新文件名。

### 可选 PVT 封装

`convert` 和 `batch` 的 `--live-photo-output pair|pvt` 选择实况照片容器，默认仍为
HEIC/HEIF + MOV 文件对。包输出示例：

```bash
xdremux convert --input IMG_001.heic --output capture.pvt --live-photo-output pvt
xdremux validate capture.pvt --json
```

PVT 是真实目录，包含 `capture.heic`、`capture.mov` 和 `metadata.plist`；
`PFVideoComplementMetadataVersionKey` 的值是字符串 `"1"`。封装保留已生成且已验证
pair 的完整字节、配对标识和封面时间，不额外编码照片或重封装 MOV。原有媒体转换策略
仍可能生成配对元数据或按需重编码照片；厂商资源遗漏继续报告，PVT 不等于无损转换。

显式包目标必须以 `.pvt` 结尾；已有文件、目录或 symlink 均保留并拒绝覆盖。
支持的 macOS/Linux 文件系统使用一次原子排他目录 rename 发布；平台或文件系统不支持时
报错，不回退到覆盖式 rename。发布失败可能保留完整 staging 包并报告路径供显式恢复，
不承诺断电持久性。将 ZIP 改名为 `.pvt` 会被拒绝。

该选择只影响 Motion Photo。混合 batch 中普通 HDR 照片保持原有输出，不能把 HEIF
伪装写到显式 `.pvt` 目标。batch JSON 将包目录列为一个输出；复用需要来源 checkpoint
及容器一致，旧文件对或来源未知的有效包均不能替代当前 PVT 请求。

支持包输出和验证，`convert`、`batch` 和 `categorize` 不将已有 PVT 或其成员当作新源；目录扫描跳过整个包，
输出和 checkpoint 写入也拒绝向已有包内部写入，包括通过别名访问的路径。

App 提供同一选择并交给 Rust 创建；仅结构有效不足以接管已有包，已有包保持不变。
当前结构测试不证明 Photos 导入或动态墙纸可用。

### OPPO 相册兼容

需要输出继续面向 OPPO Gallery 时，只使用一个产品级开关：

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic \
  --oppo-compatible
```

`--oppo-compatible` 表示“生成 OPPO Gallery 兼容输出”。XDRemux 会根据输入和目标结果自动选择内部 Gain Map 编码、metadata routing 和平台能力。

OPPO-compatible 目前只适用于 ProXDR 静态照片，不能和 Motion Photo → Live Photo 转换组合。遇到这种组合时 CLI 会明确失败，而不会静默忽略参数。

### macOS 上的 Apple 人像

不需要 Apple 私有分割能力时，使用源数据模式：

```bash
xdremux convert --input portrait.heic --output portrait_apple.heic --apple-portrait-oppo
xdremux batch --input-dir portraits/ --output-dir converted/ --apple-portrait-oppo --jobs 2
```

此模式将 OPPO 深度转换为 disparity，并保留焦点元数据、光圈、Rust 生成的 REND 和 ISO Gain Map。存在有效的 OPPO 人物、头发蒙版时才附加；空白或不可用蒙版保持缺失。不生成皮肤、牙齿、眼镜蒙版，不发送任何 Vision 请求。

**相比完整模式，人像效果会降级。** 人物和发丝边缘可能不够精细，语义光效及相关编辑可能受限。此模式仍使用 macOS 上公开的 ImageIO 和 Core Image API，不是 Linux/Windows 人像写入器。Photos 编辑行为仍取决于消费端和系统版本。

需要现有完整资源集和 Vision 语义时，使用 `--apple-portrait`；摄影风格使用 `--apple-styles`。这些选项与 `--apple-portrait-oppo` 互斥，也不能与 `--oppo-compatible` 组合。输入缺少必要的 OPPO 深度或源图数据时，在发布输出前失败。CLI 不会在失败后悄悄切换模式。

## `batch`

可以重复提供文件或目录：

```bash
xdremux batch \
  --input-dir photo_dump/ \
  --recursive \
  --output-dir converted/
```

也可以重复 `--input FILE`。目录默认不递归，使用 `--recursive` 才进入子目录。隐藏文件和 XDRemux 自己生成的 `.xdremux` 输出不会重新进入发现流程。

常用参数：

每个新转换的 Live Photo 批处理成功项在 JSON 中包含 `omitted_resource_count`，终端输出会报告非零遗漏。复用 checkpoint 不产生新转换报告，该字段为 `null`，不假定为零。规划后若其他写入者发布了相同输出，批处理发布会拒绝替换，保留已有文件。

| 参数 | 作用 |
| --- | --- |
| `--input FILE` | 添加一个输入文件，可重复。 |
| `--input-dir DIR` | 添加一个输入目录，可重复。 |
| `--recursive` | 递归扫描输入目录。 |
| `--output-dir DIR` | 指定输出目录。 |
| `--jobs N` | 最大并发转换数；必须大于 0。 |
| `--checkpoint FILE` | 指定持久 checkpoint。 |
| `--resume` | 依据 source provenance 恢复已完成工作。 |
| `--skip-existing` | 只在 provenance 与输出身份都匹配时复用已有结果。 |
| `--categorize` | 转换后直接按分类目录发布。 |
| `--live-photo-output` | 选择默认实况照片文件对或真实 PVT 目录包。 |
| `--oppo-compatible` | 对批次中的 ProXDR 静态照片请求 OPPO Gallery 兼容输出。 |
| `--apple-portrait` | 在 macOS 上请求完整人像资源集。 |
| `--apple-portrait-oppo` | 在 macOS 上使用 OPPO 深度和有效蒙版，人像效果降级。 |
| `--apple-styles` | 在 macOS 27 上一并推理生成摄影风格与纹理风格。 |
| `--apple-cinematic` | 在 macOS 27 上为普通视频推理待处理的电影效果元数据。 |
| `--json` | 输出稳定的机器可读 receipt。 |

batch 会在开始写文件前完成输出规划，避免源文件、HEIC 输出和 Live Photo MOV companion 之间发生路径碰撞。任务之间相互隔离；一个输入失败不会使已经成功发布的其他输入失去结果。

如果混合 batch 使用 `--oppo-compatible`，ProXDR 静态照片按该产品意图转换；Motion Photo 项会作为明确的逐项失败记录下来，不会假装已经应用兼容策略。

## `categorize`

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` 可以重复，目录会递归扫描。`--dry-run` 只生成计划，`--json` 输出机器可读 receipt。Live Photo 的 HEIC 与 MOV 作为同一个资产处理。

## `inspect`

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json
```

`inspect` 用于观察从源文件自动解析出的事实，例如资产类型、HDR 模式、Gain Map 数据量、Motion Photo 视频范围和 presentation timestamp。它不是转换策略配置入口。

## `validate`

```bash
xdremux validate output.heic
xdremux validate output.heic --json
```

`validate` 自动识别并验证 ISO HDR HEIF、Live Photo pair 或支持的三成员 PVT 目录。包验证检查 XML 字符串版本 plist、无歧义的普通成员文件及现有配对标识、媒体和时间契约；缺失、嵌套、symlink、重复或损坏成员会被拒绝且保持原样。这是结构验证，不是 Photos 导入验收，适合脚本和 CI 的独立转换后检查。

## 电影效果视频意图（macOS）

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
xdremux batch --input-dir videos/ --output-dir converted/ --apple-cinematic --jobs 2
```

`--apple-cinematic` 与 `--apple-styles`、`--apple-portrait` 一样，在 `convert` 和 `batch` 中启用；它与照片功能及 OPPO 兼容开关互斥。输出必须使用 `.mov` 后缀且尚不存在。单文件未指定输出时，在源文件旁生成 `SOURCE.xdremux.mov`。

命令从解码后的视频帧生成新的 Cinematic 元数据，将压缩的视频和音频样本复制到新 MOV 中。输出文件必须尚不存在，输入文件不会被替换。

发布前，Apple adapter 必须通过公共 `CNAssetInfo` 确认 `needsPreprocessing`、资源状态 `ready` 和 `isPreprocessed == false`。MOV 不包含已物化的栅格视差轨道。此状态是验收目标，不代表 Photos 编辑或渲染画质已经通过验证。

当前实现需要 macOS 27、使用 macOS 27 SDK 构建的 adapter、对应的 Apple 私有框架方法，以及受支持的推理和跟踪硬件/资源。必要的 SDK、系统、方法或资源不可用时，命令会失败。将待处理 MOV 导入 Photos 并开启电影效果，由 Photos 处理和渲染其资源。

这是实验性命令。成功识别待处理状态，不保证原生处理支持每一种输入。在 macOS 27.0.1 上，独立 1080p H.264 输入能正常处理和渲染；640×360 H.264 输入能达到待处理状态，但 Apple 预处理会以不受支持为由拒绝。命令保留压缩媒体，不进行缩放或转码。已验证范围和消费者检查见 [Cinematic 元数据封装约定](cinematic.md)。

此意图的批处理目录扫描选择 MOV、MP4、M4V、AVI、MKV 候选，媒体是否能解码由 AVFoundation 判断。不会重新发现已生成的 `.xdremux` 文件。规划器保留不冲突的 `.mov` 文件名，回执类型为 `cinematic`。Live Photo 检查点/恢复及照片分类选项不适用于此视频意图。

`--focal-length-35mm MM` 默认为 24，`--aperture F` 默认为 2.8，显式传入任一参数都需要 `--apple-cinematic`。对缺少相机几何信息的输入，这些值是生成元数据时采用的拍摄假设，不是恢复出的物理相机测量值。原生处理路径和验证方法见 [Cinematic 元数据封装约定](cinematic.md)。


`convert --cinematic-camera-estimate camera.json` 可使用绑定源视频的相机观测替代默认焦距假设，显式 `--focal-length-35mm` 优先。Rust 在原生生成前拒绝不确定或不一致的观测。首版适用于固定视角片段，`batch` 不提供此选项。可选的 [GeoCalib 研究工具](../research/cinematic_camera/README.md) 记录像素内参和不确定度，由 Rust 选择焦距。它不能恢复拍摄光圈或对焦距离。

## 产品意图与实现细节

普通 CLI 只暴露会改变用户所需结果的产品意图。源结构识别、重建算法、Gain Map layout、metadata routing、camera tail 和 codec/backend 选择由 engine/runtime 根据输入与平台能力决定，不要求最终用户配置。

需要排障时，应通过 `inspect`、结构化日志和开发测试观察自动决策，而不是把内部策略重新变成命令行参数。

`validate --apple-styles` 还会在 macOS 27 要求原生摄影风格与纹理风格读回通过。App 在跳过已有摄影风格输出前调用此检查；普通 HDR 文件不能通过该功能检查。

## 退出状态

成功、帮助和版本输出使用 `0`。运行时转换或验证失败使用 `1`。命令行语法或参数错误使用 `2`。

## 机器可读输出

`inspect --json`、`batch --json`、`categorize --json` 和 `validate --json` 提供稳定的结构化输出。默认输出仍以人类可读性为主。

## Apple 功能所有权

Photographic Styles 和 Portrait 通过 `convert` 与 `batch` 的 intent 请求。Rust 负责 policy、orchestration、数据模型、metadata、assembly、validation 和 publication。`xdremux-apple-adapter` 只调用 ImageIO、Vision、Core Image、Core ML、VideoToolbox 等平台 API，并返回事实 observation 或 primitive operation 结果。

Rust 驱动的 macOS gate 会验证结构输出和已测试的 native consumer facts。视觉等价和 Photos 交互行为仍属于独立的真机结论，不能从 parser 或 adapter probe 成功推导出来。

# XDRemux CLI 参考

[English](cli.en.md) | 简体中文

XDRemux 的产品入口是 Rust `xdremux` CLI。程序自动识别源类型、ProXDR 或 Motion Photo 转换方式及 HDR 结构。标准转换不需要选择设备代际、格式或编解码器。

完整 Mac 产品交付面向 Apple silicon（`arm64`）和 macOS 27。Rust 核心保留 Linux、Windows、macOS 构建及可移植测试。可移植构建不证明 Styles SPI 或其他 Apple 功能可用。这些功能需要文档规定的 SDK、运行时 API、硬件和原生检查。

Swift 包提供 Apple 适配器，Python 工具用于研究和训练。两者不再定义新的 CLI 产品语义，也不另设产品策略。

## 命令

| 命令 | 输入和结果 |
| --- | --- |
| `convert` | 转换一张支持的 ProXDR 静态照片或 Motion Photo。支持的 Motion Photo 自动转换为 Live Photo。`--apple-cinematic` 则选择普通视频。 |
| `batch` | 接受多个文件或目录，为每个计划项报告结果。 |
| `categorize` | 按资产类型和主要拍摄模式复制到分类目录，不转换媒体。 |
| `inspect` | 报告解析到的源数据，不转换媒体。 |
| `validate` | 检查支持的 ISO HDR HEIF、Live Photo 成员或 PVT 目录，不转换媒体。 |

```bash
xdremux --help
xdremux convert --help
```

参数定义见 [CLI 源码](../crates/xdremux-cli/src/lib.rs)。分类和验证分别由 [categorize](../crates/xdremux-cli/src/categorize.rs) 与 [validate](../crates/xdremux-cli/src/validate.rs) 模块实现。

## 终端语言

`--lang en` 选择英文，`--lang zh-CN` 选择简体中文。这个全局选项可以放在子命令前，也可以放在子命令后。

语言按以下优先级选择首个适用值：

1. `--lang`
2. `XDREMUX_LANG`
3. `LC_ALL`
4. `LC_MESSAGES`
5. `LANG`
6. 英文

空环境变量会被忽略。`zh_CN.UTF-8` 等受支持的 `zh` 区域设置选择简体中文，`C` 和 `POSIX` 选择英文。不支持的环境语言回退到英文；显式传入不支持的 `--lang` 值则返回用法错误。

语言选项改变帮助、标签和 CLI 自身的错误上下文，不改变 JSON、命令名、选项名、枚举值、路径或退出码。Clap 参数诊断详情、底层库和原生错误仍可能使用英文。

## `convert`

`--input INPUT` 必填。`--output OUTPUT` 可省略，但默认行为取决于源类型和所选功能。

| 源类型或功能 | 省略 `--output` 时 | 输入保护 |
| --- | --- | --- |
| ProXDR 静态照片，包括摄影风格或人像 | 以输入路径为输出目标。 | 转换成功后，通过原子发布替换输入。 |
| Motion Photo，默认文件对输出 | 在源文件旁选择新的 HEIC 文件名，并生成同名 MOV。 | 不替换源文件，名称不能与已有文件对成员冲突。 |
| Motion Photo，PVT 输出 | 在源文件旁选择新的 `.pvt` 目录。 | 不替换源文件。 |
| `--apple-cinematic` | 在源文件旁使用 `SOURCE.xdremux.mov`。 | 不替换源文件；目标已存在时拒绝写入。 |

> [!CAUTION]
> 需要保留 ProXDR 原图时，指定独立的 `--output` 路径。省略此选项会在转换成功后替换源文件。

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux convert --input IMG_001.jpg
```

第二条命令要求输入是支持的 Motion Photo。仅凭扩展名不能判断是否支持。

### 产品选项

以下选项适用于 `convert` 和 `batch`。

| 选项 | 条件和效果 |
| --- | --- |
| `--live-photo-output pair\|pvt` | 默认为 `pair`。选择 Live Photo 文件对或 PVT 目录，其他资产保留原有格式。 |
| `--oppo-compatible` | 为 ProXDR 静态照片请求 OPPO 相册兼容输出。Motion Photo 项拒绝此请求。 |
| `--apple-portrait` | 请求完整 Apple 人像资源集，包括 Vision 语义资源。 |
| `--apple-portrait-oppo` | 使用 OPPO 深度和已有有效蒙版，仍需 macOS ImageIO 与 Core Image，人像效果有所减少。 |
| `--apple-styles` | 在 macOS 27 上生成摄影风格和推理得到的纹理风格，需要支持的 ProXDR 源数据。 |
| `--apple-cinematic` | 在 macOS 27 上为普通视频生成待处理的电影效果元数据，属于实验性功能。 |
| `--focal-length-35mm MM` | 需要 `--apple-cinematic`，默认为 `24`，提供假定的 35 mm 等效焦距。 |
| `--aperture F` | 需要 `--apple-cinematic`，默认为 `2.8`，提供假定拍摄光圈。 |

`--oppo-compatible`、`--apple-portrait`、`--apple-portrait-oppo`、`--apple-styles` 和 `--apple-cinematic` 相互排斥。功能失败后不会静默切换模式。发布输出前，必须具备所需源数据和原生能力。

### PVT 目录输出

```bash
xdremux convert --input IMG_001.heic --output capture.pvt --live-photo-output pvt
xdremux validate capture.pvt --json
```

示例要求输入是支持的 Motion Photo。`capture.pvt` 包含 `capture.heic`、`capture.mov` 和 `metadata.plist`。plist 中 `PFVideoComplementMetadataVersionKey` 的值为字符串 `"1"`。

封装保留已生成文件对的字节、标识和封面时间，不再次编码照片或重封装 MOV。此前的转换仍可能添加配对元数据，或按需要重新编码照片。厂商资源遗漏继续报告，因此 PVT 不等于无损转换。

显式包目标必须以 `.pvt` 结尾。已有文件、目录和符号链接均保留并拒绝覆盖。受支持的 macOS、Linux 文件系统通过一次原子排他目录重命名发布；不支持的平台或文件系统直接失败，不回退到覆盖式重命名。

发布失败可能保留完整暂存包，并报告恢复路径。这不构成断电持久性保证。将 ZIP 改名为 `.pvt` 会被拒绝。

PVT 选项只影响 Motion Photo 项。混合批次中的静态照片保持原有 HDR 输出，不会把 HEIF 伪装写入显式 `.pvt` 目标。批处理 JSON 将包目录列为一个输出。

复用要求来源记录和请求容器匹配。已有文件对或来源无关的有效包不能满足 PVT 请求。`convert`、`batch` 和 `categorize` 不把已有 PVT 或包内成员视为输入资产。

目录扫描跳过 PVT 目录。输出和检查点写入拒绝包内目标，也拒绝通过路径别名写入。App 提供相同选项并交给 Rust 创建；仅结构有效不足以接管已有包。

包生成和验证不证明 Photos 导入或动态墙纸可用，详见 [Live Photo 集成](liblivephoto-integration.md)。

### OPPO 相册输出

```bash
xdremux convert --input IMG_001.heic --output IMG_001_oppo.heic --oppo-compatible
```

engine 根据源数据和输出目标选择增益图编码、元数据关系和平台操作。Motion Photo 转换遇到 `--oppo-compatible` 会失败。混合批次逐项报告这些 Motion Photo 失败。

### Apple 人像

```bash
xdremux convert --input portrait.heic --output portrait_apple.heic --apple-portrait-oppo
xdremux batch --input-dir portraits/ --output-dir converted/ --apple-portrait-oppo --jobs 2
```

OPPO 模式将源深度转换为视差，保留焦点元数据、光圈、Rust 生成的 REND 和 ISO 增益图。存在有效 OPPO 人物、头发蒙版时才附加，空白或不可用蒙版保持缺失。

此模式不生成皮肤、牙齿或眼镜蒙版，不发送 Vision 请求。与完整模式相比，人物和头发边缘可能不够准确，语义光效及相关编辑可能受限。

此模式仍需 macOS 上公开的 ImageIO 和 Core Image API，不能在 Linux 或 Windows 上写入人像资源。Photos 编辑行为取决于消费端和系统版本。

`--apple-portrait` 请求含 Vision 语义的完整资源集，`--apple-styles` 请求摄影风格。这些照片功能需要支持的 ProXDR 静态照片及相应源资源。缺少必需的 OPPO 深度或源数据时，在发布前失败。

## `batch`

```bash
xdremux batch --input-dir photo_dump/ --recursive --output-dir converted/
```

至少提供一个 `--input` 或 `--input-dir`，两者都可以重复。目录默认不递归，使用 `--recursive` 才进入子目录。扫描忽略符号链接、隐藏项、PVT 目录及已生成的 `.xdremux` 输出。

普通目录扫描选择 HEIC、HEIF、JPG、JPEG 候选文件。`--apple-cinematic` 改为选择 MOV、MP4、M4V、AVI、MKV 候选。发现候选文件不代表文件能够解码。

| 选项 | 默认值和行为 |
| --- | --- |
| `--input FILE` | 添加一个显式文件，可重复。 |
| `--input-dir DIR` | 添加一个目录，可重复。 |
| `--recursive` | 默认关闭，启用后进入输入子目录。 |
| `--output-dir DIR` | 省略时，在源文件旁生成带 `.xdremux` 后缀的输出。 |
| `--jobs N` | 必须为正整数。默认使用可用并行度，上限为 `4`，无法获取时使用 `1`。 |
| `--checkpoint FILE` | 指定检查点基础路径，实际文件名追加 `.motion-photo`。 |
| `--resume` | 来源及输出身份匹配时复用已完成 Live Photo，再重试剩余项。 |
| `--skip-existing` | 复用已有 Live Photo 输出前执行相同的来源校验。 |
| `--categorize` | 直接把转换结果发布到分类目录。 |
| `--json` | 输出一份结构化回执，替代终端文本。 |

[产品选项](#产品选项)同样适用。`--resume` 和 `--skip-existing` 要求同时指定 `--output-dir` 或 `--checkpoint`。未显式指定检查点时，文件位于输出目录中的 `.xdremux-motion-photo-checkpoint.jsonl`。

检查点复用只适用于 Live Photo，不保证跳过任意已有 HDR 或视频输出。`--apple-cinematic` 拒绝 `--resume`、`--skip-existing`、`--checkpoint` 和 `--categorize`。

规划在写入前保留源路径和输出路径，包括每组 Live Photo 的 MOV 路径。其他写入者在规划后创建相同目标时，发布拒绝覆盖。各项分别报告成功或失败；后续失败不使此前成功输出失效。

每个新转换的 Live Photo 成功项在 JSON 中报告 `omitted_resource_count`。终端文本报告非零遗漏数。复用不产生新的转换报告，因此该字段为 `null`，不会假定为零。

## `categorize`

```bash
xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run --json
```

`--input PATH` 至少提供一次，可以重复。`--output-dir DIR` 必填。输入目录递归扫描，`--dry-run` 只报告计划，不发布文件，`--json` 输出结构化回执。

命令按资产类型和主要拍摄模式复制文件。通过验证的 Live Photo 照片与 MOV 作为同一资产处理。显式符号链接输入会被拒绝，目录扫描跳过符号链接和 PVT 目录。

## `inspect`

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json
```

位置参数 `INPUT` 必填，`--json` 选择结构化输出。可用信息包括资产类型、HDR 模式、增益图数据、Motion Photo 视频范围和显示时间戳。

检查结果不改变转换策略，也不保证源文件可以转换。

## `validate`

```bash
xdremux validate output.heic
xdremux validate output.mov --json
xdremux validate capture.pvt --json
```

位置参数 `INPUT` 必填。普通验证识别支持的 ISO HDR HEIF、Live Photo 任一成员或包含三个成员的 PVT 目录，检查对应的媒体、配对和时间约定。

PVT 验证还检查 XML plist 的字符串版本及无歧义的普通成员文件。成员缺失、嵌套、符号链接、重复或损坏时拒绝验证，且不改变文件。这是结构验证，不是 Photos 导入验证。

`validate STYLES_OUTPUT --apple-styles` 额外要求在 macOS 27 上完成摄影风格与纹理风格原生回读。App 在跳过已有摄影风格输出前执行此检查。普通 HDR 文件不能通过这项功能检查。

## macOS 电影效果视频

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
xdremux batch --input-dir videos/ --output-dir converted/ --apple-cinematic --jobs 2
```

命令根据解码后的视频帧生成新元数据，把压缩的视频和音频样本复制到新 MOV。目标必须使用 `.mov` 扩展名且尚不存在，源文件不会被替换。

发布前，公共 `CNAssetInfo` 必须报告 `needsPreprocessing`、资源状态 `ready` 及 `isPreprocessed == false`。MOV 不含已生成的栅格视差轨道。待处理状态是转换验收目标，不证明 Photos 编辑或渲染质量。

适配器需要 macOS 27、使用 macOS 27 SDK 构建的程序、对应私有框架方法及受支持的推理和跟踪资源。SDK、系统、方法或资源缺失时失败。

将待处理 MOV 导入 Photos，并启用电影效果以进行原生处理。实验性流程不保证所有已识别的待处理资产都能完成处理。在 macOS 27.0.1 上，独立 1080p H.264 样本完成处理和渲染；640×360 H.264 样本达到待处理状态后，被 Apple 预处理以不受支持为由拒绝。

命令保留压缩媒体，不缩放或转码。候选视频能否解码由 AVFoundation 判断。批处理结果使用 `cinematic` 类型，并预留不冲突的 MOV 名称。

`--focal-length-35mm` 与 `--aperture` 为缺少相机几何信息的输入提供假定拍摄值，不是恢复出的物理测量值。已验证范围和独立原生消费端检查见[电影效果约定](cinematic.md)。

## 退出码与 JSON

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功、帮助或版本输出。 |
| `1` | 运行时转换、验证或输出失败。部分项目成功的批次仍可能返回 `1`。 |
| `2` | 语法、用法、输入发现或规划错误。 |

`inspect`、`batch`、`categorize` 和 `validate` 支持 `--json`，`convert` 不支持。默认输出终端文本，语言选项不改变结构化模式。

## 产品策略与 Apple 适配器

engine 和 runtime 根据输入选择源图重建、增益图布局、元数据关系、相机尾部数据处理和编解码器。通过 `inspect`、结构化结果和开发检查观察这些决策。

Rust 负责功能策略、数据模型、元数据组装、验证和发布。`xdremux-apple-adapter` 调用 ImageIO、Vision、Core Image、Core ML、VideoToolbox 及其他必需 Apple API，返回操作结果和观察到的事实。

解析器或适配器检查成功，只能证明该检查实际覆盖的约定。视觉等价和 Photos 交互编辑仍需相应环境中的独立证据。

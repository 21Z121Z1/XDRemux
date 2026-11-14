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
| `reconstruct-styles` | 保留 SDR，使用内嵌 Apple Styles 数据重建相对场景 HDR（macOS）。 |
| `hdr-look` | 从原始摄影风格照片或文件夹生成 HDR HEIC 照片（macOS）。 |

```bash
xdremux --help
xdremux convert --help
```

参数定义见 [CLI 源码](../crates/xdremux-cli/src/lib.rs)。分类和验证分别由 [categorize](../crates/xdremux-cli/src/categorize.rs) 与 [validate](../crates/xdremux-cli/src/validate.rs) 模块实现。

## `hdr-look`

```bash
xdremux hdr-look '/path/to/photo.HEIC'
xdremux hdr-look '/path/to/folder' --recursive --output-dir '/path/to/results'
```

接受多个照片或文件夹路径。默认保留原图尺寸，使用 8× HDR 亮度余量；
`--headroom` 支持 2、4、8。不指定 `--output-dir` 时，在首个输入旁创建新的
`HDR-Look-<id>` 文件夹。输出仅有 `.hdr-look.heic` 照片，文件名包含根据源路径
生成的后缀，避免不同文件夹内的同名照片冲突。不保留报告、清单或 EXR。
完成和失败信息显示在终端；`--json` 仅向标准输出返回结果。
`--dry-run` 只列出输入与目标，不调用原生处理，也不写文件。

保留原图和已有目标文件。扫描文件夹时跳过符号链接和已生成的 HDR-look 文件。
某张失败后继续处理其余照片；任何失败都会返回非零退出码。输入需包含受支持的
摄影风格内嵌数据，并要求 Apple Silicon、macOS 27 或更新版本；缺少数据会明确
报错。macOS 应用的 **摄影风格 HDR** 工作区使用同一命令，提供照片／文件夹选择、
HDR 亮度余量与输出目录设置。

算法与原生回读边界见[摄影风格 HDR 重建](styles-reconstruction.md)。
下方独立研究命令仍保留完整中间结果。

## `reconstruct-styles`

```bash
xdremux reconstruct-styles --input original.HEIC --output-dir reconstructed
```

重复 `--input` 可以批量处理，已有输出目录不会被替换。`--max-edge` 限制处理及 Gain Map 分辨率，最终 SDR 主图保留原始分辨率和压缩数据。命令返回 JSON；所需内嵌资源或原生能力不可用时明确报错。支持版本、输出文件、验证及限制见[摄影风格 HDR 重建](styles-reconstruction.md)。

`--display-headroom 2`、`4` 或 `8` 可从重建的场景参考生成单独的显示目标。它使用共同的场景参考 8x 主目标，随 headroom 减弱粗尺度空间补偿代理，并用关联的语义权重保护原图色度。同一份编码文件会检查七档 headroom。输出目录为 `.display-<headroom>x.reconstructed`，增加 `display_linear.exr`，`hdr.heic` 承载该显示目标并保留 SDR。不要求采用苹果编辑器坐标体系。

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
| 使用 `--target` 的 ProXDR 静态照片 | 在源文件旁生成新的 `*.xdremux.heic`，冲突时追加序号。 | 不替换源文件。 |
| Motion Photo，默认文件对输出 | 在源文件旁选择新的 HEIC 文件名，并生成同名 MOV。 | 不替换源文件，名称不能与已有文件对成员冲突。 |
| Motion Photo，PVT 输出 | 在源文件旁选择新的 `.pvt` 目录。 | 不替换源文件。 |
| Motion Photo，Google JPEG/HEIC 输出 | 在源文件旁生成新的 `*MP.jpg` 或 `*MP.heic` 单文件。 | 不替换源文件，也不覆盖已有文件。 |
| `--apple-cinematic` | 在源文件旁使用 `SOURCE.xdremux.mov`。 | 不替换源文件；目标已存在时拒绝写入。 |

> [!CAUTION]
> 不指定 `--target` 时，ProXDR 转换省略 `--output` 仍会在成功后替换输入。使用 `--target` 或单独指定 `--output` 可保留原图。

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux convert --input IMG_001.jpg
```

第二条命令要求输入是支持的 Motion Photo。仅凭扩展名不能判断是否支持。

### 目标预设（`--target`）

只选择输出目标，由 XDRemux 根据已识别的源资源使用现有转换路径：

```bash
xdremux convert --input capture.jpg --target apple
xdremux convert --input capture.jpg --target google
xdremux convert --input capture.heic --target oppo
xdremux batch --input-dir originals/ --output-dir converted/ --target android-standard --resume
```

| 目标 | 当前转换路径 |
| --- | --- |
| `apple` | Motion Photo 输出 Apple HEIC+MOV 文件对；ProXDR 静态照片输出标准 ISO HDR HEIC。 |
| `android-standard`（别名 `standard`）、`google` | 公开 Motion Photo 1.0。保留源 JPEG 或 HEIC 主图和原始视频字节。ProXDR 静态照片输出标准 ISO HDR HEIC。 |
| `samsung` | 公开 Motion Photo。SEF 和第二路自动播放视频留出并写明。HEIC 结果把顶层 `mpvd` 放在文件末尾，使用普通 32 位 size，载荷一直到文件尾。 |
| `oppo` | 源主图已是 JPEG 时写公开 Motion Photo，并复制已有的 `OpCamera`。HEIC 主图会规划成 JPEG 实况，但在静图重编码会改写或丢掉资源时停止发布。25.17.35 的 `isOlivePhoto` 和 17.10.62 的 `saq.w` 都要求 `image/jpeg`。17.10.62 的 v3 签名者 SHA-1 是 `39bad40aa2730a1914e27939c1c04529df320c98`。一加和 realme 使用这个目标。 |
| `vivo` | 已有两个 UUID 盒的仓库样本按原字节保留。其他 JPEG 实况写成 JPEG 加保留的 MP4：`MotionPhoto=1`、`Item:Padding=0`，视频起点是文件大小减去 `Item:Length`。ExtInfo 写在该起点之前，类型是 `motionphoto0001…`，`imageTime` 是离源演示时间最近的 30fps 帧。HEIC 主图在重编码前停止。这里的 10.0.10.42 是 Y04s/W20 包。这不是实机识别结果。 |
| `xiaomi` | 公开 Motion Photo，版本恰好为 1。4.3.1.15-global 在此条件下用文件大小减去 MotionPhoto 项的 Length，不读 Padding。不写 `MiCamera` 和 `MicroVideo`。 |
| `huawei` | 视频位于文件末尾时，原样保留前缀和视频，并追加 40 字节尾。播放信息是毫秒 `start:time`：`seekTo` 用源演示时间截到毫秒，自动停止延时用保留的 `mvhd` 时长截到毫秒。不写空白字段。 |

`convert` 和 `batch` 都支持 `--target`。JPEG 实况仍输出 JPEG，HEIC 实况仍输出 HEIC。`--target oppo` 遇到 HEIC 主图会规划 JPEG 实况，并在重编码静图之前停止。主图和视频都不转码。目标名称不代表相册识别、HDR 激活或分享行为。`--motion-report-json` 会写明容器、从源文件沿用的字段、没有重写的资源，以及 `native_vendor_claim: false`。这些输出都还没有在本仓库的设备上验收。

为兼容旧调用，未使用 `--target` 时仍支持 `--motion-target`、`--live-photo-output` 和 `--oppo-compatible`；这些选项不能与 `--target` 同时使用。Apple 编辑开关继续作为显式高级请求：如果指定 `--target`，必须为 `apple` 才能使用这些开关。目标预设不会自动启用实验性人像、摄影风格或电影效果推断。

### Google Motion Photo 输出（`convert` 和 `batch`）

```bash
xdremux convert --input photo.jpg --motion-target google-jpeg
xdremux convert --input photo.heic --motion-target google-heic --output captureMP.heic
xdremux convert --input apple.heic --motion-video apple.mov --motion-target google-heic --output captureMP.heic --motion-report-json
xdremux convert --input photo.jpg --motion-target google-jpeg --motion-video-compat preserve --motion-report-json
```

`--motion-target` 可选 `apple-pair`、`apple-pvt`、`google-jpeg`、`google-heic`，不能与原有的 `--live-photo-output` 同时使用。两个选项都不指定时，仍然默认输出 Apple 文件对。批处理只有显式指定目标时才改变原有 Apple 输出行为。

Google 目标通过 LibLivePhoto 的 `plan → execute` 写入器输出一个自包含文件。转换保留源 JPEG/HEIC 静态图像编码、视频容器字节和精确呈现时间。输出文件名须以 `MP.jpg`、`MP.jpeg` 或 `MP.heic` 结尾，并与目标类型相符；未指定输出路径时自动生成。不进行隐式转码或丢弃无法映射的资源：静态图像格式不匹配、时间戳无法精确表达、厂商资源无法保留时，发布前直接失败。文件结构通过检查不等于已在特定 Android 相册中通过实机验证。

`--motion-video apple.mov` 显式提供与输入 HEIC 关联的 Apple MOV 文件。程序核验两者的配对标识，再直接封装成 Google HEIC；照片和视频均不重新编码。此选项必须与 `--motion-target google-heic` 一起使用。不会仅凭同名文件自动推断 Apple 配对；原有 Apple 输出和批处理默认行为保持不变。

`--motion-video-compat` 仅接受 `preserve`，也是默认行为。完整视频容器（包括 `ftyp` 品牌）逐字节保留，运行时拒绝任何视频字节或呈现时间改动。该路径不执行视频转码或重封装，因此不能保证不兼容的编码或轨道布局在目标相册中播放。

`--motion-report-json` 将普通成功提示替换为 JSON：列出每项源资源的编号、角色、类型、字节长度、全部保留/重写处置、生成资源、视频品牌操作和严格验证证据。任何遗漏或不可映射资源都会阻止发布。发布前还要核对辅助图像（含 Gain Map、HEIF 图像项）的提取字节、Gain Map 的 XMP 属性、静态图像色彩与展示元数据、视频字节或显式品牌改写以及精确呈现时间。现有 SDR 输出路径不受影响。

### 产品选项

以下选项适用于 `convert` 和 `batch`。

| 选项 | 条件和效果 |
| --- | --- |
| `--live-photo-output pair\|pvt` | 默认为 `pair`。选择 Live Photo 文件对或 PVT 目录，其他资产保留原有格式。 |
| `--hdr-profile compatible\|bt2020-pq\|bt2020-hlg` | 默认为 `compatible`。为 ProXDR 静态照片显式选择 BT.2020 替代图像编码。Motion Photo 项拒绝显式 BT.2020 配置。 |
| `--oppo-compatible` | 为 ProXDR 静态照片请求 OPPO 相册兼容输出。Motion Photo 项拒绝此请求。 |
| `--apple-portrait` | 请求完整 Apple 人像资源集，包括 Vision 语义资源。 |
| `--apple-portrait-oppo` | 使用 OPPO 深度和已有有效蒙版，仍需 macOS ImageIO 与 Core Image，人像效果有所减少。 |
| `--apple-styles` | 在 macOS 27 上生成摄影风格和推理得到的纹理风格，需要支持的 ProXDR 源数据。 |
| `--apple-cinematic` | 在 macOS 27 上为普通视频生成待处理的电影效果元数据，属于实验性功能。 |
| `--focal-length-35mm MM` | 需要 `--apple-cinematic`，覆盖自动记录元数据或 GeoCalib 焦距选择。 |
| `--cinematic-camera-estimate PATH` | 仅用于 `convert`。提供绑定源视频的显式相机观测，优先级低于显式焦距、高于自动选择。 |
| `--aperture F` | 需要 `--apple-cinematic`，默认为 `2.8`，提供假定拍摄光圈。 |

`--oppo-compatible`、`--apple-portrait`、`--apple-portrait-oppo`、`--apple-styles` 和 `--apple-cinematic` 相互排斥。功能失败后不会静默切换模式。发布输出前，必须具备所需源数据和原生能力。

显式 `--hdr-profile` 选项不能与上述兼容性或 Apple 编辑选项组合。BT.2020 配置接受经验证采用 sRGB 或 Display P3 色域、sRGB 传递函数的 SDR 基础图像。导出器检查 ICC 变换及并存的色彩声明。不支持的色彩编码、裁剪几何、非方形像素，以及不支持的额外矩阵或 LUT 映射，会在发布前失败。

BT.2020 配置保留 SDR 压缩资源及其色彩数据。基础图像、Gain Map 和 HDR 派生图像使用相同顺序的旋转与镜像变换。派生图像采用基础图像的编码尺寸，并声明 BT.2020 与 PQ 或 HLG。Gain Map 的压缩色彩属性独立保留。`compatible` 继续选择现有相机或显示兼容布局。

```bash
xdremux convert --input IMG_001.heic --output IMG_001_pq.heic --hdr-profile bt2020-pq
xdremux convert --input IMG_001.heic --output IMG_001_hlg.heic --hdr-profile bt2020-hlg
```

CLI 不生成推测的母版、内容亮度或动态元数据。Rust HEIF 组装 API 接受调用方基于源数据提供的、经过检查的 `mdcv`、`clli` 和注册 `it35` 元数据。注册载荷中的静态数据必须与派生图像属性一致。动态载荷接受语法检查，并保留原始字节。动态显示映射不属于此导出器的处理范围。JPEG APP8 载荷编解码器不提供双 JFIF 写入器。

### PVT 目录输出

```bash
xdremux convert --input IMG_001.heic --output capture.pvt --live-photo-output pvt
xdremux validate capture.pvt --json
xdremux validate output.heic --semantic --json
xdremux validate IMG_001_pq.heic --hdr-profile bt2020-pq --decode-hdr --json
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
xdremux batch --input-dir photo_dump/ --output-dir android/ --target google --resume --json
xdremux batch --input-dir photo_dump/ --output-dir google/ --motion-target google-jpeg --resume --json
xdremux batch --input photo.heic --output-dir google/ --motion-target google-heic --motion-video-compat preserve --resume
```

至少提供一个 `--input` 或 `--input-dir`，两者都可以重复。目录默认不递归，使用 `--recursive` 才进入子目录。扫描忽略符号链接、隐藏项、PVT 目录及已生成的 `.xdremux` 输出。

普通目录扫描选择 HEIC、HEIF、JPG、JPEG 候选文件。`--apple-cinematic` 改为选择 MOV、MP4、M4V、AVI、MKV 候选。发现候选文件不代表文件能够解码。

| 选项 | 默认值和行为 |
| --- | --- |
| `--input FILE` | 添加一个显式文件，可重复。 |
| `--input-dir DIR` | 添加一个目录，可重复。 |
| `--recursive` | 默认关闭，启用后进入输入子目录。 |
| `--output-dir DIR` | 省略时，在源文件旁生成带 `.xdremux` 后缀的输出。 |
| `--target apple\|android-standard\|google\|samsung\|oppo\|vivo\|xiaomi\|huawei` | 选择平台/厂商预设。混合 Android Motion Photo 批处理会按每张主图的实际 JPEG 或 HEIC 编码选择输出。 |
| `--motion-target apple-pair\|apple-pvt\|google-jpeg\|google-heic` | 选择 Motion Photo 目标，默认 Apple 文件对。Google 输出单个 `*.xdremuxMP.jpg` 或 `*.xdremuxMP.heic`，不转码静态照片。 |
| `--motion-video-compat preserve` | 仅 Google 目标。可省略；`preserve` 是唯一模式且为默认值，不改写品牌或转码。 |
| `--jobs N` | 必须为正整数。默认使用可用并行度，上限为 `4`，无法获取时使用 `1`。 |
| `--checkpoint FILE` | 指定检查点基础路径，实际文件名追加 `.motion-photo`。 |
| `--resume` | 来源及输出身份匹配时复用已完成的 Apple 或 Google Motion Photo，再重试剩余项。 |
| `--skip-existing` | 复用已有 Motion Photo 输出前执行相同的来源校验。 |
| `--categorize` | 直接把转换结果发布到分类目录。 |
| `--json` | 输出一份结构化回执，替代终端文本。 |

[产品选项](#产品选项)同样适用。`--resume` 和 `--skip-existing` 要求同时指定 `--output-dir` 或 `--checkpoint`。未显式指定检查点时，文件位于输出目录中的 `.xdremux-motion-photo-checkpoint.jsonl`。

检查点复用只适用于 Motion Photo 转换，不保证跳过任意已有 HDR 或视频输出。检查点升级为版本 3，仍可读取旧的 Apple 文件对/PVT 记录。Google 记录包含输出目标、视频策略及完整输出 SHA-256。恢复时核验源大小与摘要、输出路径与摘要以及严格解析结果。变更视频策略后另选文件名；已有输出遭到修改则报错且不覆盖；输出缺失时可沿用记录中的路径重建。`--apple-cinematic` 拒绝 `--resume`、`--skip-existing`、`--checkpoint` 和 `--categorize`。

Google 目标逐项拒绝 ProXDR 静态照片、JPEG/HEIC 格式不匹配及无法保留厂商资源的输入。当前不支持 `google-avif`，目录扫描也不收集 AVIF。批处理不会根据同名文件推断 Apple HEIC+MOV 配对；须用 `convert --motion-video` 明确提供文件对。

规划在写入前保留源路径和输出路径。Apple 文件对还保留 MOV 路径；Google 单文件只保留自身路径。目录重新扫描时忽略已生成的 `*.xdremuxMP.jpg` 和 `*.xdremuxMP.heic`。其他写入者在规划后创建相同目标时，发布拒绝覆盖。各项分别报告成功或失败；后续失败不使此前成功输出失效。

新转换的 Apple Live Photo 在 JSON 中报告 `omitted_resource_count`；Google 输出通过无损资源校验后报告 `0`。终端文本报告非零遗漏数。复用不产生新的转换报告，因此该字段为 `null`，不会假定为零。Google 回执使用 `kind: "motion-photo"`，只列一个输出路径。

可移植测试不能证明原生相册兼容性；宣称 Google Photos、OPPO 相册或三星相册支持前，按 [Android 实机验证矩阵](validation/android-motion-gallery-matrix.md)测试。

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

`--semantic` 额外检查最终 HEIF 中的数值色调映射元数据和替代图像色彩编码声明，并拒绝矛盾的元数据与图像通道数。`--hdr-profile bt2020-pq` 或 `bt2020-hlg` 还检查所选色彩编码、源色彩信息、编码尺寸、方向属性顺序，以及存在的受支持 HDR 元数据载荷。这些选项适用于独立 HEIF 文件。

`--decode-hdr` 需要 `--hdr-profile`。它从最终文件解码基础图像与 Gain Map，保留分量精度，不降低至八位。解码器保留编码坐标。重建先对 Gain Map 反伽马，再做共位双线性插值，在声明的应用色域中处理增益与偏移量，最后转换至线性 BT.2020。JSON 报告实际解码精度及相对 RGB 的最小值和最大值，并保留色域转换产生的负分量。

`valid: true` 表示所选检查通过。可选 HDR 元数据缺失时会如实报告。只有使用 `--decode-hdr` 才检查像素重建。相对线性重建不能证明绝对显示亮度、动态色调映射、与另一源文件的 SDR 等价性或设备识别。PQ 和 HLG 传递函数参考值的测试与显示适配分开进行。

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

两个入口都自动选择焦距：显式 `--focal-length-35mm MM` 优先，然后是 ExifTool 读取的固定 35mm 等效焦距，字段缺失或未知时调用已安装的 GeoCalib helper。只有物理焦距还不够，不再默认代入 24 mm。依赖缺失、元数据冲突或模型观测被拒绝时，在发布输出前失败。`--aperture F` 仍是独立假设，默认为 2.8。显式传入任一参数都需要 `--apple-cinematic`。支持的标签、限制和原生验证见 [Cinematic 元数据封装约定](cinematic.md)。


`convert --cinematic-camera-estimate camera.json` 保留为高于自动选择的显式观测输入，显式 `--focal-length-35mm` 优先。正常使用不需要 JSON 参数，`batch` 会逐个解析输入。[GeoCalib 模型适配器](../model_adapters/geocalib/README.md) 需要一次性本地安装，目前支持固定视角 SDR 视频。准入和焦距选择由 Rust 持有。它不能恢复拍摄光圈、对焦距离或未知 EIS 裁切；模型不确定度较低也不等于物理精度已验证。

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

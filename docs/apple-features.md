# Apple 功能

[English](apple-features.en.md) | 简体中文

摄影风格和 Apple 人像已经是 XDRemux 中由 Rust 持有的产品 intent。它们不是第二套产品栈。

Canonical 公开产品是 Rust `xdremux` CLI。Swift target 只作为无法跨平台完成的 Apple framework adapter，例如 ImageIO consumer probing、Vision observation 和 VideoToolbox encoding。

## 当前可用性

标准 HDR、OPPO 兼容输出、Motion Photo → Live Photo、batch、分类、inspect 和 portable validation 已属于 canonical Rust 产品。

摄影风格和 Apple 人像通过 `convert`/`batch` 的产品 intent 表达，不新增 Apple 专用 CLI 子命令，也不把 adapter 或 solver 的底层控制暴露为公开契约。

Rust 持有 feature request/result model、routing、fallback、validation policy、metadata synthesis、assembly 和 publication lifecycle。Apple-native 代码只执行 Rust 请求的平台 operation，并返回事实 observation。

## 平台边界

CLI 使用以下结构：

```text
xdremux CLI
    ↓
Rust runtime
    ↓
Rust engine policy
    ↓
portable providers + Apple platform adapter
                         ↓
          ImageIO / Core Image / Vision /
          Core ML / AVFoundation / ...
```

`xdremux-apple-adapter` 是由 Rust 产品消费的可分发平台组件。它不是用户 CLI，也不持有产品 policy。

CLI/runtime 使用有版本号、生命周期有界的 helper-process protocol。每次转换复用一个延迟启动的 adapter session。超时、崩溃或帧格式错误会终止该 session。transport 保持为 Rust runtime 的私有实现，因此更换 transport 不会改变 engine 或 CLI 语义。

## Rust 持有的 policy

Rust 已把用户层 Apple feature intent 建模为两个事实：摄影风格和人像。旧 Swift 的 producer、donor、backend、research control 不作为产品配置重新暴露。

第一项真实 Apple adapter operation 是 ImageIO auxiliary-resource probing。adapter 只报告 observation，例如：

- ISO Gain Map 是否存在；
- disparity 是否存在；
- Portrait Effects Matte 是否存在；
- skin、hair、teeth、glasses semantic matte 是否存在。

adapter 不回答“这是不是有效人像输出”这类业务问题。该判断由 `xdremux-engine` 中的 `AppleImageAuxiliaryFacts` 和 Portrait resource contract 持有。

新增操作必须沿用同一边界：返回最窄且足够的 framework fact 或 operation result，再由 Rust 持有 policy。

## 摄影风格迁移

Rust 持有 style generation 语义、constrained search、source-bound policy、key1/property-list synthesis、graph assembly、validation policy 和 publication。adapter 只执行 Rust runtime 请求的 framework observation 或 encoding primitive。

研究型 producer、model experiment、donor diagnostic 和 RAW experiment 继续作为 research tooling，不定义公开 CLI 契约，也不构成第二套 runtime。

### 同一开关生成纹理风格

`--apple-styles` 与 App 的“Apple 摄影风格”开关会一并生成纹理风格，不增加独立的 Texture 选项。App 在跳过已有输出前调用 `validate --apple-styles`；普通 HDR 输出不能通过此检查。此功能需要已在 macOS 27 验证的私有 Vision 和 CMImaging API。API 缺失或任一步骤失败时，整个摄影风格事务在发布前报错。

Rust 请求 ImageIO 生成已校正方向、长边最多 1024 像素的 sRGB 推理图。此图仅用于分析；输出保留完整尺寸主图、HDR Gain Map、Styles Linear Thumbnail 和已有项目载荷。分析不能恢复 RAW 或 Apple 拍摄时的处理数据。

Vision 提供人脸关键点、有符号姿态角、人物实例与语义蒙版。通过最大总重叠匹配，每个人脸只关联一个独立实例。Rust 为每人调用原生 Mattify、UnderEyeBrighten 和 SkinSmoothStandalone 统计生产端，共享同一套实际几何与蒙版。调用顺序为 setup、prewarm、prepare、calculateStats、process、GPU 等待及最后一次 finishProcessing。

Rust 只合并原生实际返回的统计。成功但为空的可选统计组会省略。没有匹配人脸时写入空人物数组，不创建模板人物。鼻、唇、眉和手部保护使用原生关键点或手势检测推导的区域；本次派生编辑禁用耳部和纹身保护。这不等同于完整的相机 FSINC 生产端。

Rust HEIF 写入器向已有摄影风格输出添加 v3 Texture plist、独立编码的蒙版和精确的实例 XMP 引用键。调参配置仅选择派生编辑，不替换 EXIF 相机身份、曝光、镜头、时间或拍摄 UUID。搬移后逐项核对原有载荷。原子发布前，由原始 Apple 解析器读回并校验人物数量、引用键、几何和统计。

在 macOS 27 运行 `python3 scripts/apple/check_texture_styles.py --all-fixtures --output <dir>`，验证真实 ProXDR fixture、批处理、主图与 Gain Map HEVC 保持一致，以及失败时保留已有输出。此前 SDR 研究候选已有用户报告 Photos 激活成功。该反馈不代表每份新生成的完整尺寸 HDR 产品输出、效果响应、保存及重新打开均已通过设备测试。

### SmartStyle 编辑状态

生成的 Standard 风格资源还需要在 Apple MakerNote 标签 84（`0x0054`）中记录 SmartStyle 状态。系数 plist 是另一个独立输入；解析成功不能证明照片具备完整编辑状态。Rust 定义旧 Swift writer 使用的所需的整数编辑字段（包括 Standard 纹理状态），ImageIO 原语将这些字段合并到已有 MakerApple 字典，不新增相机身份或 UUID，并保留字典中其他条目和未修改的字段。

发布前，Rust 暂存最终组装的完整字节，并要求 ImageIO 回读其中的整数字典字段。必需状态缺失或变化时，转换在替换已有输出前失败。旧 adapter 即使忽略新增的可选元数据请求，也不能静默发布不完整的风格文件。此原生回读不代表 Photos 控件、Tone/Color 响应或保存重开已通过；这些行为仍需实机证据。

### Linear Thumbnail 的 HDR 参照

Styles Linear Thumbnail 是由 SDR base image 与 Gain Map 重建出的 scene-linear Display P3 HDR 的 Apple Log 编码。Rust 在 base image 的原始分辨率上应用 Gain Map：先在码值域对 Gain Map 做双线性重采样，再按源文件实际携带的 ISO 21496-1 / Apple tmap 参数重建每个样本，包括 log2 最小/最大值、26A434 `_flexMap` 解码所用的 gamma 倒数指数、逐通道偏移，以及 alternate image 的 headroom 权重。缩到 1024 像素级缩略图发生在这次重建之后，并在线性光下做面积盒式滤波。随后 Rust 将每个通道除以 Styles plist key `4` 中相同的 `baseline_exposure`，再应用 Apple Log。此资源不再直接复制 SDR 图像码值。

Apple Log 传递函数、[0, 1] 码值范围和现有辅助色彩标记保持不变。编码器是 macOS 26A434 指数解码器（`_appleLogToLinear` / `sw_appleLogToLinear`）的逆变换。该解码器不使用二次 toe，因此编码器也不使用。相机端编码器 toe 尚未核实。此变换保留 Display P3 原色，不会将图像转换成 Apple Log 视频所用的 BT.2020，也不会把缩略图改标成 sRGB、PQ、HLG 或线性。

系数使用已解码的显示方向场景。RGB16 Linear Thumbnail 编码前，Rust 撤销主图完整的旋转与镜像序列。存储的 Linear Thumbnail 和 Style Delta Map 使用主图编码几何，并保留其 `irot` 和 `imir` 的相对顺序。Gain Map 或 Styles 图像的宽高比与主图编码尺寸不一致时，转换会被拒绝。`encoding_gain` 属于渲染器元数据，不是第二次曝光除法。

编码原语接收 16 位 Apple Log 码值。VideoToolbox 将这些码值转成 10 位 video-range 4:2:0，并编码为 Main10 HEVC。码值在进入编码前不再量化成 RGB8，因此编码器可以使用 10 位范围。超过 SDR 白的 HDR 数值仍然保留。该参照仍来自已经渲染过的 HDR look，不能恢复源文件已丢失的 RAW 或拍摄时处理信息。

同一个 26A434 consumer `PISemanticStyleFilter` 也决定风格元数据如何生成：

- key 3 是一条 256 节点曲线，逐通道独立采样。横坐标是乘过 plist base gain 之后的线性缩略图码值 `min(S, 1)`，不做亮度合并，也不做分位数归一化。曲线是这些主图通道观测均值的单调投影。consumer 额外曝光 `E` 是渲染参数，不写入曲线。
- key `c` 和 `d` 保持 32×32。每格保存一个 guide 的面积均值：`c` 是主图，`d` 是线性缩略图。consumer 用自己的 guided filter（`r = 1`，`eps ≈ 0.05`）上采样，再计算 `q = mix(mix(d, c, l_b), mix(d, c, l_f), m)`。这两张图不再乘固定仿射，也不直接乘到亮度上。消费 `q` 的后续 shader 阶段没有反解，因此这些均值不是解出来的局部调性。
- key 1 是 x-major 的 12×9×8×10×3 half 数组，二次基为 `[1, R, G, B, R², RG, RB, G², GB, B²]`。每个 spotlight 格子和每个亮度 plane 分开拟合。拟合方向是 remove-style：多项式读取显示中的主图，目标是缩略图码值。局部样本无法确定的系数保持恒等残差。plane 权重使用已核实的 `inputIsLinear = false` 公式，计算在 `f64` 中进行，不模拟 half，也不应用可选的 3×3 空间滤波。

背景和前景 LTM 都为 1 时，也就是已捕获的 tone=0 设置，semantic-tone 阶段返回主图，不读取缩略图或曲线。因此 tone=0 的渲染测不到 key 3 和缩略图。Photos 的 Tone ±1 与 Color ±1 滑块位置不是源事实。可移植公式检查是 `apple_style_semantic_tone_channel`。

原生 tone 检查与产品路径分开。它检出 `21Z121Z1/Photographic_Style_Research` 的 `37dd551`，并在 `xcode-27` 镜像上运行 `consumer-model/harness/run_tone_sweep.py`。该 harness 只设置 `inputToneBias`，没有颜色控制。扫描的 tone 为 −1、−0.75、−0.5、−0.35、−0.15、0、0.15、0.35、0.5、0.75 和 1。Apple 样本是 `Apple_Native_Samples/IMG_0032.HEIC`、`IMG_0033.HEIC`，以及 `research/vm-native/samples/IMG_2600.HEIC`、`IMG_2604.HEIC` 和 `IMG_2608.HEIC`。转换后的三个 ProXDR 夹具使用同一组 tone。`IMG_0032.HEIC` 在 tone 0 必须报告 0 个不同的 half 分量，之后的渲染才可采信。报告记录平均亮度、逐通道均值、`|L(−1)−L(0)| / |L(+1)−L(0)|`、单调性，以及与 tone 相关的 plist 字段。工作流是 `.github/workflows/research.yml`。

原生 consumer 回归使用 RGB UHDR 和单色 LHDR 夹具，回读编码后的缩略图，检查 HDR Log 目标以及主图 HEVC 是否保持不变。运行命令为 `bash scripts/apple/check_rust_style_consumer.sh`。Photos 导入、编辑、保存和重开仍需单独的设备证据。baseline exposure 和 base gain 仍使用现有的源分布校准；这次改动不替换那组系数。

### 临时的系统 gain map

默认编码器仍然写入 XDRemux 自己的 log gain。`XDREMUX_SYSTEM_GAIN_MAP=1` 是一条单独的原型路径。Rust 重建 scene-linear Display P3 HDR，用临时 guided filter 生成 HDR look，并用 local weight 1 保持 SDR look，这个权重就是恒等变换。然后把两张图交给 Apple adapter。adapter 调用 `CIContext.writeHEIFRepresentationOfImage`（`format` 为 `.RGBA8`，色彩空间为 Display P3），并通过 `kCIImageRepresentationHDRImage` 传入 half 线性 HDR。headroom 使用头文件方法 `-imageBySettingContentHeadroom:`（macOS 26）。该头文件没有给出这个方法的 Swift overlay 名称。`kCIImageRepresentationHDRGainMapAsRGB` 设为 true，要求 Core Image 计算颜色比值。Rust 不提供 `kCIImageRepresentationHDRGainMapImage`。

Styles 系数与 Linear Thumbnail 复用真正提交给系统编码器的 SDR/HDR 图像对，包括 SDR 量化和提交的 half-float HDR，不会继续用原 OPPO Gain Map 的通道或 scale 参数解释新 Gain Map。回读失败会在原子替换已有输出前拒绝测量。

这些名称和可用版本来自 macOS 27 SDK 头文件（`CIContext.h`、`CIImage.h`、`CGImage.h`、`CGImageSource.h`、`CGImageDestination.h`、`CGImageProperties.h`），不是猜出来的 selector。`writeHEIFRepresentationOfImage` 从 macOS 10.13.4 起可用。HDR image 选项和 `contentHeadroom` 从 macOS 15 起可用，`imageBySettingContentHeadroom` 从 macOS 26 起可用。Rust 送出的 headroom 是交给编码器的 half 数值的 `max(1, measured peak)`。头文件不允许 `(0, 1)` 区间内的 headroom。ImageIO 的 `kCGImageDestinationEncodeGenerateGainMapWithBaseImage` 从 macOS 16 起存在，但它的字典值类型没有文档，因此原型不调用它。

回读使用 `kCGImageSourceDecodeRequest` = `kCGImageSourceDecodeToHDR`，并把 `kCGImageSourceGenerateImageSpecificLumaScaling` 设为 false，然后渲染成 extended-linear Display P3 half。它同时报告 `kCGImageAuxiliaryDataTypeHDRGainMap` 和 `kCGImageAuxiliaryDataTypeISOGainMap`。系统文件实际包含哪一种是测量结果，不是头文件事实。Styles 组装器仍然要求恰好一个 `tmap` item 及其 `dimg` 图。如果系统文件没有这张图，组装会失败关闭，报告记下这个冲突。Photos 导入和编辑仍需实机证据。

这个 guided filter 不是 `guided.py` 的移植；公开研究仓库里没有该文件。半径 1、epsilon 0.05、以 `f16` 保存的半尺寸系数、HDR local weight 0 和 SDR local weight 1 都是临时值。`XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE` 限制原型几何尺寸。工作流把它设为 256。

## Apple 人像迁移

Rust 持有 Portrait preflight、OPPO block parsing、focus/orientation policy、JPEG/container logic、Gain Map policy、REND generation、auxiliary-manifest construction、feature routing、output naming、validation policy 和 atomic publication。ImageIO 只报告 auxiliary-resource facts，adapter 只执行 Rust transaction 要求的 Apple framework operation。

完整人像模式通过独立的 image request handler 逐个提交 Vision 分割请求，避免把不同分割请求类型一起提交到曾在 macOS 27 beta 上观察到崩溃的 compound request 路径。请求的语义资源和输出校验要求保持一致。任何请求失败都会终止本次操作；adapter 不会重试该请求，也不会改用源数据模式。

### 源数据模式和效果降级

`convert` 和 `batch` 支持 `--apple-portrait-oppo`。此模式不调用 Vision，包括其私有分割 SPI。它使用公开的 ImageIO 和 Core Image 操作，因此仍需要 macOS。

| 资源 | 源数据模式 |
| --- | --- |
| ISO Gain Map | 从源文件保留。 |
| Disparity、焦点、光圈、REND | 由 Rust 根据 OPPO 深度、拍摄数据和源元数据生成。 |
| Portrait Effects Matte | OPPO 人物平面存在可信前景时才附加。 |
| Hair matte | OPPO 头发平面存在可信前景时才附加。 |
| 皮肤、牙齿、眼镜蒙版 | 保持缺失，不生成替代蒙版。 |

相比完整的 `--apple-portrait` 模式，人像效果会降级。源蒙版分辨率较低或缺失时，主体分离和发丝细节可能降低，语义光效及相关编辑可能受限。仅包含深度的输出可以满足源数据模式的资源契约，但不等同于完整语义资源集。OPPO pet 平面不会被冒充成人物或头发蒙版。

源数据模式不会回退到 Vision。缺少必要深度或源元数据时，在发布输出前报错。完整模式保持原有严格资源要求。两种模式共用 Rust 转换和发布路径。

源数据模式回归使用两张真实 OPPO 人像样张，以 ImageIO 核对单文件及串行、并行批处理输出，并拒绝任何 Vision 操作。它还验证预处理失败时保留已有输出。运行命令为 `python3 scripts/apple/check_rust_cli_oppo_portrait.py`。

## Live Photo

普通 Motion Photo → Live Photo 已经是 Rust 产品能力，不应绕回 Apple capability adapter。

如果未来组合功能需要对 Live Photo 静态照片执行 Apple-only operation，Rust 仍必须持有 Live Photo asset lifecycle、pair identity、publication 和 validation ordering。Apple adapter 只接收它真正需要执行的窄平台 operation。

## 兼容规则

产品级 compatibility rule 属于 Rust。例如 Apple editing feature 是否能与 OPPO-compatible output 组合、源资产是否具备 Portrait editing 所需资源，以及组合请求如何 atomic publish，都应由 Rust 决策。

扩展 CLI 时，不要把这些策略移入 adapter protocol。

## 验证与验收

把证据分成三类：

1. 结构证据证明 HEIF/MOV resource 和 metadata 存在且可解析；
2. 原生 framework 证据证明被测试的 Apple framework 能接受或暴露预期资源；
3. 真机证据证明特定真实设备和 Apple Photos 版本上的行为。

涉及 Apple Photos 交互式编辑的结论，不能用结构证据替代真机证据。

Canonical completion gate 要求 Rust workspace 和真实 Rust → Apple adapter handshake 在 macOS 上通过。feature-specific gate 驱动 Rust CLI 后查询 Apple consumer facts。结构和 native-framework evidence 本身不等于 visual equivalence 或 Photos 真机验收。

CLI 迁移验收覆盖唯一的 Rust 命令入口、媒体转换、与源数据匹配的资源契约和 Apple adapter。macOS App 集成属于独立范围。源数据模式的效果降级是已记录的产品取舍。Photos 交互式编辑仍是独立消费端结论；CLI 通过不代表 Photos 效果或编辑界面完全一致。

## 研究材料

仓库仍保留摄影风格研究代码和 `ReverseKey1Ensemble` 等可选模型。它们属于研究/训练资产，不是产品模式。需要时见[模型卡](../research/oppo_styles/models/ReverseKey1Ensemble.model-card.md)。

# Apple 编辑功能

[English](apple-features.en.md) | 简体中文

XDRemux 通过 Rust `xdremux` CLI 提供 Apple 编辑功能。Rust 负责产品策略和文件发布，Swift/Objective-C 适配器执行所需的 Apple 框架操作。

本文说明摄影风格、纹理风格和 Apple 人像。实验性视频命令单独见[电影效果指南](cinematic.md)。

## 可用范围与命令

完整产品、App 和 Apple 适配器要求 Apple silicon（`arm64`）和 macOS 27 或更新版本。适配器不提供 Intel 构建，并在报告能力或执行操作前检查运行环境。

可移植的 Rust 核心仍保留 Linux、Windows 和 macOS 测试。这些检查不能证明各平台均可使用 Apple 功能。即使系统和架构受支持，仍需验证具体 API、源资源、GPU 执行结果和最终文件。

| `convert` 或 `batch` 选项 | 请求的结果 |
| --- | --- |
| `--apple-styles` | 为 ProXDR 照片生成摄影风格和推理所得的纹理风格。 |
| `--apple-portrait` | 生成包含完整语义资源集的 Apple 人像。 |
| `--apple-portrait-oppo` | 使用 OPPO 深度与已有源蒙版生成 Apple 人像，不调用 Vision。 |
| `--apple-cinematic` | 为普通视频生成实验性的待预处理电影元数据。详见[电影效果指南](cinematic.md)。 |

这些选项互斥，并且都不能与 `--oppo-compatible` 组合使用。它们不增加独立的 Apple CLI 子命令。适配器、供体、后端和求解器的底层控制不属于公开产品选项。当前组合规则由 [CLI 解析器](../crates/xdremux-cli/src/lib.rs)定义。

标准 HDR、OPPO 兼容输出、Motion Photo 转换、批处理、分类、检查和可移植验证仍属于 Rust 产品操作。

## 平台边界

| 层级 | 职责 |
| --- | --- |
| Rust CLI | 表达用户请求的产品操作。 |
| Rust 引擎 | 定义资源模型、兼容规则和验证策略。 |
| Rust 运行时 | 执行操作、组装资源、验证输出并发布文件。 |
| Apple 适配器 | 调用 ImageIO、Core Image、Vision、Core ML、AVFoundation、VideoToolbox 等所需框架。 |

`xdremux-apple-adapter` 是 Rust 使用的可分发组件，不是公开转换 CLI。

运行时通过带版本号的辅助进程协议与适配器通信，限制消息大小和操作时长。每次转换复用一个按需启动的会话。超时、崩溃或消息帧错误会终止该会话。传输方式仍是运行时的私有实现。

ImageIO 辅助资源检查只报告具体观测结果，包括 ISO Gain Map、视差、Portrait Effects Matte，以及皮肤、头发、牙齿和眼镜蒙版。适配器不判断完整产品输出是否有效。`xdremux-engine` 通过 `AppleImageAuxiliaryFacts` 和资源契约作出该判断。

新增适配器操作必须返回 Rust 所需的框架事实或操作结果，产品决策仍由 Rust 负责。实现见 [apple_adapter.rs](../crates/xdremux-runtime/src/apple_adapter.rs) 和 [apple.rs](../crates/xdremux-engine/src/apple.rs)。

## 摄影风格

Rust 根据源文件生成风格，负责约束搜索、key 1 与 plist 合成、资源图、验证和发布。适配器提供框架观测和编码操作。

研究生成器、模型实验、供体诊断和 RAW 实验仍属于研究工具，不定义另一套运行时或公开转换方式。

### 同一开关生成纹理风格

`--apple-styles` 和 App 的「Apple 摄影风格」开关在同一次操作中生成纹理风格，不增加独立纹理选项。App 复用已有输出前会运行 `validate --apple-styles`。普通 HDR 输出不能通过这项检查。

纹理生成需要已在 macOS 27 测试的私有 Vision 和 CMImaging API。API 缺失或原生操作失败时，整个风格事务在发布前失败。

分析图与输出承担不同用途：

- ImageIO 生成方向已校正、最长边不超过 1024 像素的 sRGB 分析图。
- 输出保留完整主图、HDR Gain Map、Styles Linear Thumbnail 和已有项目载荷。
- 分析不能恢复 RAW 数据或 Apple 拍摄时的处理信息。

Vision 提供人脸关键点、有符号姿态角、人物实例和语义蒙版。系统按蒙版总重叠最大原则，将每张实测人脸匹配到独立人物实例。Rust 为每个匹配人物请求 Mattify、UnderEyeBrighten 和 SkinSmoothStandalone 统计。所有效果使用同一组推理几何和蒙版。

原生调用顺序为 `setup`、`prewarm`、`prepareToProcess:`、`calculateStats`、`process`、GPU 同步等待和最后的 `finishProcessing`。分析、推理、统计和最终回读分别使用类型化请求与结果。Rust 运行时的[适配器客户端](../crates/xdremux-runtime/src/apple_adapter/texture.rs)负责解析原生报告和命名工作文件。

仅有辅助进程确认消息，不能证明统计已经生成，也不能允许发布。GPU 完成状态、生命周期、输入绑定和最终字节回读检查仍然适用。

Rust 只合并原生操作实际返回的统计，省略成功但为空的可选统计组。没有匹配人脸的场景写入空人物数组，不创建模板人物。鼻、唇、眉和手部保护采用原生关键点或手势推导的区域。本次派生编辑禁用耳部和纹身保护，不复现完整的相机 FSINC 生成端。

Rust HEIF 写入器添加版本 3 的 Texture plist、单独编码的蒙版和精确的逐实例 XMP 引用键。调参配置选择的是派生编辑，不替换 EXIF 相机身份、曝光、镜头、日期或拍摄 UUID。

资源搬移后，写入器逐项检查原有载荷。原子发布前，Apple 解析器必须回读到相同的人物数量、引用键、几何和统计。

在 macOS 27 上运行样本检查：

```bash
python3 scripts/apple/check_texture_styles.py --all-fixtures --output /absolute/path/new-texture-results
```

检查覆盖真实 ProXDR 转换、批处理、主图与 Gain Map HEVC 保持不变，以及失败时保留已有输出。此前的 SDR 研究候选有用户报告 Photos 激活成功。这不能证明每份完整分辨率 HDR 产品输出、效果响应、保存和重新打开均已通过。

实现见 [apple_texture.rs](../crates/xdremux-runtime/src/apple_texture.rs) 和 [XDTextureNative](../Sources/XDTextureNative)。

### SmartStyle 编辑状态

Standard 风格资源集还需要 Apple MakerNote 标签 84（`0x0054`）中的 SmartStyle 状态。系数 plist 是独立输入，解析成功不能证明照片的编辑状态完整。

Rust 定义必需的整数编辑字段，包括 Standard 纹理状态。ImageIO 将这些字段合并到已有 MakerApple 字典，不新增相机身份或 UUID。其他条目及未修改字段保持不变。

发布前，Rust 暂存组装后的完整文件，并要求 ImageIO 回读整数字典字段。必需字段缺失或变化时，转换在替换已有输出前失败。旧适配器即使忽略新增的元数据请求，也不能静默发布不完整的风格文件。

原生回读不能证明 Photos 控件、Tone/Color 响应或保存后重新打开的行为。这些结论需要设备证据。

### Linear Thumbnail 的 HDR 参照

Styles Linear Thumbnail 保存由 SDR 主图和 Gain Map 重建出的场景线性 Display P3 HDR，经 Apple Log 编码后的码值，不是 SDR 图像码值的副本。

Apple 适配器通过 Core Image，在主图原始分辨率上读取 SDR 和展开后的 HDR 表示。框架解释源 ICC/nclx 和 `tmap` 色度，包括增益图应用基色、gamma、偏移及亮度余量，并以扩展线性 Display P3 RGBA32F 返回两幅图像。Rust 保留浮点 RGB 样本及位图自上而下的行序。该路径不经过 RGB8 中转，也不再无条件执行 sRGB 传递函数解码。

读回每次最多渲染 128 行，将浮点字节流式写入临时文件，再原子发布。读取下一幅表示前会释放当前表示的渲染资源，以限制中转内存，同时保留分辨率和精度。

重建后，面积盒式滤波在线性光下将图像缩小到 1024 像素级缩略图。Rust 将每个通道除以 Styles plist key `4` 中的 `baseline_exposure`，再应用 Apple Log。

Apple Log 传递函数、`[0, 1]` 码值范围和辅助色彩标记保持不变。编码器是 macOS 26A434 指数解码器 `_appleLogToLinear` / `sw_appleLogToLinear` 的逆变换。该解码器不使用二次暗部曲线，因此此编码器也不使用。相机端编码器的暗部曲线尚未核实。

缩略图使用从源颜色空间转换得到的 Display P3 原色，不使用 Apple Log 视频采用的 BT.2020，也不将缩略图改标为 sRGB、PQ、HLG 或线性。

系数使用解码后按显示方向排列的图像。RGB16 编码前，Rust 撤销主图完整的旋转和镜像序列。存储的 Linear Thumbnail 与 Style Delta Map 使用主图编码几何，并保留 `irot` 和 `imir` 的相对顺序。

Gain Map 或 Styles 项目的宽高比不匹配时，转换失败。`encoding_gain` 是渲染器元数据，不是另一个曝光除数。

编码器接收 16 位 Apple Log 码值。VideoToolbox 将其转换为 10 位视频范围的 4:2:0，再编码为 Main10 HEVC。输入不经过 RGB8 量化，因此编码器可以使用 10 位范围。超过 SDR 白的 HDR 值仍有对应表示。

该参照来自已经渲染的 HDR 外观，不能恢复源文件已经丢失的 RAW 或拍摄时处理信息。

### 风格系数字段

26A434 的 `PISemanticStyleFilter` 消费端决定 XDRemux 如何构造以下字段。

| 字段 | 存储形式 | 构造方法与限制 |
| --- | --- | --- |
| Key `3` | 一条 256 节点曲线，逐通道独立采样。 | 横坐标是应用 plist 基础增益后的 `min(S, 1)`，不进行亮度合并或分位数归一化。 |
| Key `c` 和 `d` | 两张 32×32 引导图。 | 每格保存面积均值。`c` 表示主图，`d` 表示线性缩略图。 |
| Key `1` | 按 x 优先排列的 12×9×8×10×3 半精度数组。 | 每个 spotlight 网格和亮度平面分别使用二次基拟合。常数项和线性项共用该风格的线性界限，二次项使用其二次界限。Standard 为 1/4 和 3/16，Gold 为 3/4 和 5/8，其余风格为 3/8 和 1/4。 |

Key `3` 是实测主图通道均值的单调投影。消费端的额外曝光 `E` 属于渲染参数，不包含在该曲线中。

下游 `LinearGTCImage` 统计和报告中的拟合误差使用实际写出的 key 3 节点，按实测 macOS 约定在固定 65535 尺度执行 inverse-sRGB，不使用量化前的拟合曲线。256 个解码节点保留 f32，忽略末尾重复的线格式字。iPhone 固件按终点归一化的 half 表属于另一条绑定。

消费端使用自己的引导滤波器（`r = 1`、`eps ≈ 0.05`）上采样 key `c` 和 `d`，然后计算：

```text
q = mix(mix(d, c, l_b), mix(d, c, l_f), m)
```

这些图不采用固定仿射缩放，也不直接与亮度相乘。消费 `q` 的后续着色器阶段尚未反解，因此这些均值不代表局部色调变换已经求解。

Key `1` 使用基 `[1, R, G, B, R², RG, RB, G², GB, B²]`。多项式读取显示中的主图。调用方可以把主图拟合到另一幅 RGB 图像；局部样本无法确定的系数保持恒等残差。平面权重采用已核实的 `inputIsLinear = false` 公式，并以 `f64` 计算，不模拟半精度。可选的 3×3 空间滤波不参与计算。

拟合按正在写入的风格把每个残差限制在对应界限内。`Standard` 和 `Gold` 使用各自的界限。Amber、Bright、Natural，以及任何其他或无法识别的名称，使用第三组界限。plist 不保存该界限。

产品写入的 Standard 载荷是恒等多项式。Photos 把 Key 1 作用在显示中的主图上。主图到线性缩略图码值的比例，包括 3 通道增益图的逐通道比例，放不进 Standard 的系数界限。被截断的二次拟合会使用跨通道项，并画出 12×9 的色块网格。线性缩略图和 Key 3 仍然承载场景与全局色调曲线。

背景和前景局部色调映射（LTM）均为 1 时，语义色调阶段返回主图，这对应已捕获的 `tone=0` 设置。该阶段不读取缩略图或曲线，因此零色调渲染无法测量这两项资源。

Photos 的 Tone ±1 和 Color ±1 滑块位置不是源文件事实。可移植公式检查为 `apple_style_semantic_tone_channel`。基准曝光和基础增益仍采用源图像分布校准；重建不替换这些校准系数。

实现见 [apple_photographic_styles.rs](../crates/xdremux-engine/src/apple_photographic_styles.rs) 和 [apple_styles.rs](../crates/xdremux-runtime/src/apple_styles.rs)。

### 原生摄影风格检查

原生色调实验独立于产品转换。它检出 `21Z121Z1/Photographic_Style_Research` 的 `37dd551`，通过 [research.yml](../.github/workflows/research.yml) 在 `xcode-27` 镜像上运行 `consumer-model/harness/run_tone_sweep.py`。

测试只设置 `inputToneBias`，没有颜色控制。色调取值为 −1、−0.75、−0.5、−0.35、−0.15、0、0.15、0.35、0.5、0.75 和 1。

Apple 样本为 `Apple_Native_Samples/IMG_0032.HEIC`、`IMG_0033.HEIC`，以及 `research/vm-native/samples/IMG_2600.HEIC`、`IMG_2604.HEIC` 和 `IMG_2608.HEIC`。三个转换后的 ProXDR 样本使用相同色调取值。只有 `IMG_0032.HEIC` 的零色调检查报告 0 个不同的半精度分量后，其他渲染结果才可采信。

报告记录平均亮度、通道均值、单调性、色调相关 plist 字段，以及 `|L(−1)−L(0)| / |L(+1)−L(0)|`。

产品消费端回归采用 RGB UHDR 和单色 LHDR 样本，解码已编码的缩略图，检查 HDR Log 目标及主图 HEVC 是否保持不变。在符合要求的 Mac 上运行：

```bash
bash scripts/apple/check_rust_style_consumer.sh
```

设置 `XDREMUX_STYLES_ACCEPTANCE_DIR` 时，检查保留真实的 v15 产品 HEIC、原生纹理资源和各样本的 `linear-thumbnail-roundtrip.json` 报告。这些是诊断文件，不是 Photos 验收证据。

只读检查器根据声明的 file/idat 存储位置读取资源，并验证数据范围，不把产品修改为 v16。独立的版本号研究对照仍是单字节实验。Photos 导入、编辑、保存和重新打开需要设备检查。

## 临时系统 Gain Map 方案

默认编码器写入 XDRemux 自己的对数增益。`XDREMUX_SYSTEM_GAIN_MAP=1` 选择独立的原型实现。

Rust 重建场景线性 Display P3 HDR，并使用临时引导滤波器生成 HDR 外观。SDR 局部权重 1 对应恒等变换，保留 SDR 外观。随后 Rust 将两张图像提交给 Apple 适配器。

适配器对 SDR 图像调用 `CIContext.writeHEIFRepresentationOfImage`，采用 `.RGBA8` 格式和 Display P3 色彩空间。半精度浮点线性 HDR 通过 `kCIImageRepresentationHDRImage` 提交，亮度余量通过 `-imageBySettingContentHeadroom:` 设置。头文件没有为该方法提供 Swift 映射名称。

`kCIImageRepresentationHDRGainMapAsRGB` 为 true，要求 Core Image 计算颜色比值。Rust 不提供 `kCIImageRepresentationHDRGainMapImage`。

风格系数与 Linear Thumbnail 复用实际提交的 SDR/HDR 图像对，包括 SDR 量化和半精度浮点 HDR，不会使用原 OPPO 增益图的通道或缩放参数解释新图。回读失败时，测量在替换已有输出前失败。

### API 要求

API 名称及引入版本来自 macOS 27 SDK 头文件。已检查的头文件包括 `CIContext.h`、`CIImage.h`、`CGImage.h`、`CGImageSource.h`、`CGImageDestination.h` 和 `CGImageProperties.h`。

| API 或属性 | SDK 引入版本或限制 |
| --- | --- |
| `writeHEIFRepresentationOfImage` | macOS 10.13.4。 |
| HDR 图像选项和 `contentHeadroom` | macOS 15。 |
| `imageBySettingContentHeadroom` | macOS 26。不允许 `(0, 1)` 范围内的亮度余量。 |
| `kCGImageDestinationEncodeGenerateGainMapWithBaseImage` | macOS 16。字典值类型没有文档，因此原型不调用它。 |

这些版本不降低完整产品的 arm64/macOS 27 要求，也不证明私有摄影风格和纹理风格 SPI 可用。Rust 根据实际提交的半精度浮点值发送 `max(1, measured peak)`。

### 回读与尚未确认的行为

回读将 `kCGImageSourceDecodeRequest` 设为 `kCGImageSourceDecodeToHDR`，将 `kCGImageSourceGenerateImageSpecificLumaScaling` 设为 false，然后渲染为扩展线性 Display P3 半精度浮点值。

报告同时包含 `kCGImageAuxiliaryDataTypeHDRGainMap` 和 `kCGImageAuxiliaryDataTypeISOGainMap`。输出是否具有这些资源，需要实际测量，不能根据头文件声明推断。

风格组装仍要求恰好一个 `tmap` 项目及其 `dimg` 图。系统输出缺少该图时，组装失败，报告记录冲突。Photos 导入和编辑仍需要设备证据。

该引导滤波器不是 `guided.py` 的移植，公开研究检出中没有这个文件。临时参数包括半径 1、epsilon 0.05、以 `f16` 保存的半尺寸系数、HDR 局部权重 0 和 SDR 局部权重 1。`XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE` 限制原型几何尺寸，工作流将其设为 256。

实现见 [system_heif_hdr.rs](../crates/xdremux-runtime/src/system_heif_hdr.rs) 和 [CoreImageHeifHdr.swift](../Sources/XDRemuxAppleAdapter/CoreImageHeifHdr.swift)。

## Apple 人像

Rust 负责源数据预检、OPPO 数据块解析、焦点与方向策略、容器处理、Gain Map 策略、REND 生成和辅助资源清单，也负责路由、输出命名、验证及原子发布。ImageIO 报告辅助资源事实。适配器执行 Rust 事务需要的框架操作。

完整模式为每个 Vision 分割请求使用独立的图像请求处理器，避开曾在 macOS 27 beta 观察到崩溃的混合分割请求提交方式。所需语义角色和输出验证要求不变。

任一请求失败都会终止操作。适配器不重试该请求，也不改用 OPPO 源数据模式。

### 源数据模式与效果限制

`--apple-portrait-oppo` 使用 OPPO 源数据，不调用 Vision 或其私有分割 SPI。由于仍使用公开 ImageIO 和 Core Image 操作，该模式依然需要 macOS。

| 资源 | 源数据模式 |
| --- | --- |
| ISO Gain Map | 从源文件保留。 |
| 视差、焦点、光圈和 REND | Rust 根据 OPPO 深度、拍摄数据和源元数据生成。 |
| Portrait Effects Matte | 仅在 OPPO 人物平面具有可信前景时添加。 |
| 头发蒙版 | 仅在 OPPO 头发平面具有可信前景时添加。 |
| 皮肤、牙齿和眼镜蒙版 | 保持缺失，不生成替代蒙版。 |

与完整的 `--apple-portrait` 模式相比，此模式的效果受限。蒙版分辨率较低或缺失时，主体分离和发丝细节可能减少，语义光效及相关编辑也可能受限。

只有深度的输出可以满足源数据模式契约，但不等于完整语义资源集。OPPO 宠物平面不会被改标为人物或头发蒙版。

源数据模式从不回退到 Vision。缺少必需深度或元数据时，操作在发布前失败。完整模式保留严格资源要求。两种模式共用 Rust 转换和发布方式。

原生回归采用两张真实 OPPO 人像样本。ImageIO 检查单文件转换及串行、并行批处理输出，测试会拒绝任何 Vision 操作。测试还检查预检失败后已有输出是否保持完整。在符合要求的 Mac 上运行：

```bash
python3 scripts/apple/check_rust_cli_oppo_portrait.py
```

## Live Photo 与兼容策略

普通 Motion Photo 转 Live Photo 属于 Rust 操作，不使用 Apple 适配器。未来若对 Live Photo 照片增加 Apple 专用操作，配对身份、验证顺序和发布仍必须由 Rust 负责。

产品兼容性决策保留在 Rust，包括源资源要求、与 OPPO 兼容输出的冲突，以及受支持请求的发布规则。扩展适配器协议时，不能把这些决策移入适配器。

## 验证与验收

| 证据类型 | 能够证明的内容 |
| --- | --- |
| 结构 | 预期 HEIF/MOV 资源与元数据存在且可解析。 |
| 原生框架 | 被测 Apple 框架在该环境接受或暴露预期资源。 |
| 设备 | 指定设备和 Apple Photos 版本上的实际操作成立。 |

完成检查要求 Rust 工作区及真实的 Rust 与适配器握手在 macOS 上通过。功能检查调用 Rust CLI，再检查原生消费端结果。结构和原生检查不能证明视觉等同或 Photos 设备验收。

CLI 验收覆盖命令、媒体转换、与源文件匹配的资源契约和 Apple 适配器。App 集成属于独立范围。源数据模式的效果限制仍是产品取舍，Photos 交互式编辑需要单独证据。

`ReverseKey1Ensemble` 等研究模型属于训练资产，不是产品模式。详见[模型卡](../research/oppo_styles/models/ReverseKey1Ensemble.model-card.md)。

# 产品架构与媒体契约

[English](architecture.en.md) | 简体中文

Rust 工作区负责 XDRemux 的产品行为。Apple 适配器执行 Rust 请求的框架操作。Python 用于研究和训练。

本文说明组件职责和输出安全规则。命令用法见 [CLI 参考](cli.md)，首次转换见[项目 README](../README.md)。

## 组件职责

| 组件 | 职责 |
| --- | --- |
| `crates/xdremux-cli/` | 解析公开命令，并选择需要执行的操作。 |
| `crates/xdremux-engine/` | 描述源文件事实、用户意图、可用能力和转换策略。 |
| `crates/xdremux-runtime/` | 执行转换计划、验证结果并发布输出文件。 |
| Rust 媒体 crate | 解析元数据、处理 HDR、编码图像，以及读写容器。 |
| LibLivePhoto | 提供可移植的 Motion Photo 和 Live Photo 文件格式实现。 |
| `Sources/XDRemuxAppleAdapter/` | 调用 Apple 框架，通过 Rust 定义的协议返回观测结果。 |
| `Sources/XDTextureNative/` 和 `Sources/XDCinematicNative/` | 为适配器实现原生纹理风格和电影效果操作。 |
| `apps/macos/XDRemuxApp/` | 提供 Mac 界面并调用产品操作。 |
| `xdremux_py/` | 提供独立于产品运行时的研究和训练工具。 |

### Rust 产品栈

Rust 负责 ProXDR 元数据解析、ISO/TS 21496-1 Gain Map 转换、HEIF 组装、分类、批处理、验证和发布。Apple 人像与摄影风格的资源模型和事务策略也由 Rust 定义。

LibLivePhoto 提供源文件检查、资源选择、精确呈现时间和配对验证。XDRemux 保留 HDR 处理、编解码器选择、时间推断、几何处理和文件发布策略。详见 [LibLivePhoto 接入契约](liblivephoto-integration.md)。

CLI 在进入普通 HDR 处理前自动识别受支持的 Motion Photo。两类转换采用不同的输出安全规则，具体见 [CLI 参考](cli.md)。

### Apple 框架操作

完整 Apple 产品要求 Apple silicon（`arm64`）和 macOS 27 或更新版本。App 与原生安装包采用相同的部署目标。适配器在报告能力前拒绝不受支持的运行环境。

适配器调用 ImageIO、Vision、Core Image、VideoToolbox 等所需的 Apple 框架。它返回框架观测结果，或写入 Rust 请求的资源，不决定产品兼容策略。

Swift 包只发布 `xdremux-apple-adapter` 可执行文件。原生纹理风格和电影效果目标为该适配器提供支持，不构成另一套公开转换 CLI。详见 [Package.swift](../Package.swift) 和 [Apple 功能文档](apple-features.md)。

可移植的 Rust 测试仍覆盖 Linux、Windows 和 macOS。这些测试不能证明原生框架在各平台可用，也不能证明设备上的实际表现。

### 研究工具

`xdremux_py/` 不参与 Rust 运行时，也不定义公开转换方式。研究结果可以为产品修改提供依据，但不能替代产品测试或原生验收。

### 模型原语

`model_adapters/geocalib/` 是独立的本地推理组件。Rust runtime 通过 ExifTool 读取已记录的等效焦距，只有缺失时才调用模型。Python helper 返回绑定源视频的像素内参和不确定度。来源优先级、固定相机准入、等效焦距换算、原生生成和发布均由 Rust 持有。helper 不构成第二套转换器或用户 CLI。固定版本环境与权重需要单独安装，不上传媒体。


## 媒体契约

### 标准 HDR

标准转换把厂商 HDR 元数据转换为 ISO/TS 21496-1 表示。在所选处理方式允许时，转换保留源文件的压缩图像数据。这不代表每项源资源都能保持不变。

ProXDR 转换保留主图 HEVC 数据及源 ICC/nclx。可移植 RGB Gain Map 编码器使用 BT.601 全范围 YCbCr；增益图网格及图块声明 nclx `(2, 2, 6)`。前两项按 [HEIF 色调映射派生](https://www.iso.org/standard/89044.html)要求设为未指定，不描述重建 HDR 的色域。标准 RGB 输出使用 4:4:4；OPPO 兼容输出保留 4:2:0。

tmap 默认色度为 BT.2020 PQ。它描述派生图像：解码器在 `use_base_colour_space` 指定的基色中应用增益，再按照 [ISO 21496-1](https://www.iso.org/standard/86775.html) 附录 B 转换为替代编码，不要求存储一幅 PQ 主图。采用 ISO APP2 元数据时，转换在源文件提供 alternate ICC 时保留它；Android hdrgm 由基础图像配置文件定义 HDR 基色，不把 Gain JPEG 的 ICC 当作 alternate 色度。ISO alternate ICC 缺失时，即使源标志为零，也保留基础图像的增益应用基色（C.4.7）。无 ICC 的 JPEG RGB 使用随附的标准 sRGB ICC，让编码后的基础图像具备明确色度（C.4.4）。

Styles 重建和 HDR Look 在线性 Display P3 中拟合场景像素。生成新的 RGB Gain Map 时，Rust 先把 SDR 和 HDR 像素都转换到线性 BT.2020，再在该空间中计算新的通道增益。这些输出选择替代图像的基色（`use_base_colour_space=0`），并保留 tmap 的 BT.2020 PQ 声明。公共偏移覆盖负线性分量，避免在对数比值计算前裁剪它们。即使原样保留的 SDR 主图使用 sRGB 或其他配置文件，增益仍在其计算所用的工作空间应用。此行为不改变 ProXDR 增益图的应用空间。

ISO JPEG 输入中，Exif `ColorSpace=1` 即使在存在 ICC 时也选择 sRGB（C.4.4）。其他 Exif 值在源文件提供 ICC 时保留它。Android hdrgm 输入沿用现有 ICC 选择行为。

### JPEG 增益图元数据选择

运行时在基线图像之后定位完整的增益 JPEG，并在解析元数据前核对已声明的 GainMap 长度。基线 JPEG 中只含版本的 ISO APP2 不是增益图参数。在已定位的增益 JPEG 中，有效 ISO APP2 优先于 Adobe hdrgm XMP，包括参数冲突的情形。这采用 [Android Ultra HDR v1.1 的兼容建议](https://developer.android.com/media/platform/hdr-image-format#iso_21496-1_compatibility)，不是 ISO 自身定义的跨格式优先级。有效 ISO 无需先解析 XMP；只有 ISO 缺失时才进入 XMP 路径。

ISO 已存在但损坏或版本不受支持时，转换失败；即使 XMP 有效或 Motion Photo 没有映射 GainMap 资源，也不能转为 XMP 或 SDR 成功输出。错误会指出 ISO 失败原因。这是 XDRemux 的保守转换策略。ISO C.2.3 对通用阅读器的规定则是忽略未知最低版本并显示基线图像。转换失败会保留源文件和已有输出。

ISO 校验先用精确的 64 位整数交叉乘法比较有符号增益区间和无符号基线／替代 headroom，再生成浮点观测值。逐分量最小增益必须不大于最大增益，gamma 必须为正，两个 headroom 必须不相等。HDR 基线允许 headroom 递减。有理数分母为零、最低版本不受支持、必需字段截断和保留标志非零都会失败。只要最低版本和必需字段受支持，就继续接受更高的 writer version 和可选尾部字节。XMP 保留 Android 色彩绑定和默认值，单独检查 `HDRCapacityMin` 非负且 `HDRCapacityMax` 严格大于它。现有真实 JPEG 样本与这些 capacity 检查没有冲突。

ISO 必需字段的原始有理数对、元数据分量数和 writer version 直接传入 HEIF 写入器，不做十进制舍入、钳位或 exp2/log2 往返。受支持的可选尾部可以丢弃。公开浮点观测值不能独立于这些源字节修改。XMP 和新生成的场景元数据使用有界连分数：增益／偏移使用有符号分子，gamma／headroom 使用无符号分子。投影误差不得超过 max(1e-12, abs(value) * 1e-9)，非零值不能变成零；越界或无法表示时失败。写出的 tmap 会重新解析并执行精确数值校验。兼容 XMP 使用能够往返浮点值的格式；ISO headroom 使用最多 40 位小数并精确选择方向，避免不同分数变成相同文本。替代 ICC 缺失时，C.4.7 要求使用基线应用基色，即使源标志为零。`tmap` 恰好引用 `[base, gain]`，各图像分别携带色彩属性。增益压缩保留全范围 BT.601 nclx `(2, 2, 6)`；`tmap.colr` 描述替代表示，可承载源替代 ICC。[样本审计](validation/20261028-iso-jpeg.md)记录兼容性范围和验证命令。

ISO 替代 ICC 检查头部、长度、类别、RGB/PCS、标签边界／对齐／唯一性、保留字节和重叠。允许标签完全共享相同数据。配置文件必须包含描述、版权、介质白点，以及矩阵／TRC 或设备到 PCS 的变换数据。有界的纯 Rust CMM（由 Cargo.lock 固定版本的 `moxcms`）解析配置文件，并以 f64 执行相对比色的正反向变换。检查关闭 CICP 覆盖；非有限或退化变换失败。HDR 替代表示（替代 log2 headroom 大于零）必须带 CICP；RGB CICP 的矩阵为 0。

CICP 一致性检查使用 9³ RGB 网格，加上各 257 点的中性色阶和独立通道色阶。在归一化线性 RGB 中，实际 ICC 变换与 CICP 声明编码的逐分量差异不得超过 0.002 + 0.005 * abs(reference)。这是产品准入容差，不是 ICC 合规阈值，也不是最终图像误差预算。未指定的 CICP 字段沿用矩阵／TRC 配置文件数据；部分字段未指定的 LUT 绑定无法验证，会明确失败。限幅 CICP 范围标志是合法值，但其 RGB 归一化需要明确的分量位深；当前替代 ICC 路径将其报告为不支持，不猜测映射。HLG 一致性使用 CMM 的名义系统 gamma 1.2 参考；采用其他黑电平／OOTF 约定的配置文件可能被拒绝为超出此策略。

存在但无效或不支持的配置文件会在发布前失败，不能当成缺失或替换为猜测的色域。通过检查的原字节保留到 `tmap.prof`；准入 CMM 不对产品像素重新着色。基线 ICC 准入和 XMP-only 增益 ICC 处理保留现有行为。有限采样和单一 CMM／渲染意图不能证明所有配置文件、渲染意图或原生消费者等价。[后续验证](validation/20261028-color-integrity.md)记录反例和剩余缺口。

### 色彩精度与覆盖范围

受色彩管理的线性 RGBA32F 工作路径和明确的 P3→BT.2020 转换保留扩展范围及负分量，直到增益构造。最终生成的增益图仍为 8 位，并使用质量 90 的有损 HEVC。gamma 为 1、增益跨度为 S EV 时，仅舍入即可产生最多 S / (2 * 255) EV，即 2^(S / (2 * 255)) - 1 的增益倍率误差。允许的最大 24 EV 跨度对应约 3.32%，尚未计入压缩。这是增益误差，不是最终场景 RGB 或感知色差的上界；偏移和接近黑位时的相消也会影响结果。

编码器回归在固定平滑 RGB 渐变上测量量化加实际 HEVC 编解码，分别覆盖 4:4:4 和 4:2:0，报告码值误差以及按 24 EV 换算的最大值和 p99。阈值用于防止该图案上的回退，不能约束任意边缘、图像或原生显示。源保真、元数据完整性、增益压缩、原生颜色读回和设备显示仍是不同证据。现有定向 fixture 样本和 CI 通过率不能建立总体“99% 正确”的结论。

### Live Photo

默认输出由一张 HEIC/HEIF 照片和一个 MOV 组成，两者共享资产标识符。转换器把解析出的源封面时间映射为 Apple `still-image-time`。

MOV 写入器直接复制压缩的视频和音频样本，不进行解码和重新编码。运行时在发布前比较源视频和输出视频的媒体载荷。源文件包含 Gain Map 且输出方式支持时，照片写入器会保留该 Gain Map。

`LivePhotoOutput` 可以把同一组已验证文件放入真实的 PVT 目录。封装过程不再次编码或重封装媒体。Rust 负责包结构、验证和发布，App 传递输出类型并展示计划生成的容器。

### 发布

Live Photo 配对输出只有在两个最终文件均已发布，并通过字节回读检查后，才算转换成功。运行时先在私有暂存目录写入并同步已验证的完整配对文件，再用不会替换已有目录项的原子硬链接占用 MOV 名称，最后发布照片。

已有输出项及无关文件、目录保持不变。遵循同一协议的写入者竞争同一个 MOV 名称。同名但后缀分别为 `.heic` 和 `.heif` 的输出也遵循此规则。

两个文件名不构成一个整体原子事务。第二个硬链接失败时，命令报告已发布的 MOV 路径，并保留暂存目录。最终文件的字节回读失败时，也保留暂存目录并记录已观察到的最终目录项。

运行时尝试从已校验的缓冲区写入独立的 `recovery.heic` 和 `recovery.mov`。它同步这两个文件，并将文件字节与缓冲区比较。只有这些操作成功后，错误信息才确认完整的独立恢复文件对已保留。否则，错误信息报告 `recovery-unverified`。原来的 `capture.*` 文件可能与受损的最终文件共享 inode。

错误信息确认独立恢复文件对已通过检查时，对报告路径中的 `recovery.heic` 执行 `xdremux validate`。移动任一成员前，先检查两个最终路径。已有输出检查会保留并拒绝来源未知的不完整配对或无效配对。名称看似临时文件，并不能证明当前操作有权删除它。

配对发布要求同一文件系统支持硬链接。文件系统不支持时，发布失败并保留原始 I/O 错误；不宣称支持 FAT/exFAT。该协议无法阻止外部写入者在回读后替换文件。暂存文件已经同步，也不保证断电后目录项仍然持久保存。

PVT 采用另一种文件系统操作。运行时在本次操作拥有的私有目录内写入已验证媒体和固定 XML 版本 plist，再通过排他目录重命名占用最终包路径。已有目录即使为空，也不会被替换。

重命名前，运行时解除旧暂存路径的自动删除责任，避免删除后来出现在旧路径的新目录项。发布失败时，完整包保留在错误报告中的恢复路径。

macOS 和 Linux 缺少排他重命名能力时，发布会失败。其他核心平台可以读取和验证受支持的包结构，但不会改用能够覆盖已有包的重命名操作。PVT 结构有效不能证明断电持久性、Photos 导入或墙纸支持。

批处理复用已有输出时，要求源文件来源记录和容器身份一致。来源未知的有效配对或包，不能作为另一份输入的转换结果。App 没有持久保存包来源的检查点，因此不会自动接管已有 PVT。

实现见 [pair_publication.rs](../crates/xdremux-runtime/src/live_photo/pair_publication.rs) 和 [live_photo_package.rs](../crates/xdremux-runtime/src/live_photo_package.rs)。

## 延伸阅读

- [Apple 功能文档](apple-features.md)说明平台要求与证据边界。
- [开发文档](development.md)说明构建和组件职责。
- [测试政策](quality/testing.md)规定修改需要的验证证据。
- [验证操作指南](validation/README.md)说明如何将检查绑定到已提交的版本。
- [测试样本说明](../fixtures/README.md)规定源媒体的身份和来源要求。

## 历史审计

[2026 年 5 月 11 日的 ISO 一致性审计](history/iso-conformance-audit-20260511.md)记录当时的实现。其中的旧路径不能作为当前架构参考。

保留历史审计中的测量结果。新的规范符合性研究取代旧结论时，增加新的带日期记录。当前文档遵循[技术写作规范](style-guide.md)。

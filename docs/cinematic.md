# 待预处理的 Cinematic MOV

[English](cinematic.en.md) | 简体中文

`xdremux convert --apple-cinematic` 从普通视频生成新的待预处理 Cinematic MOV。验收目标是 Apple 公共 API 的 `needsPreprocessing` 状态。命令不保证推理出的电影效果与原始相机效果相同。

## 命令与运行条件

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
```

该开关与人像、摄影风格一样，可用于 `convert` 和 `batch`，并与照片功能及 OPPO 兼容开关互斥。输出必须使用 `.mov` 后缀且尚不存在；单文件未指定输出时，在源文件旁生成 `SOURCE.xdremux.mov`。输入文件保持不变。Rust runtime 在生成前后计算输入哈希，通过不会替换现有输出的硬链接发布候选文件。

当前实现需要 macOS 27、使用 macOS 27 SDK 构建、可用的 Apple 推理资源，以及 adapter 使用的 Portrait/Cinematic 私有框架方法。adapter 在调用前检查每个必要私有 Objective-C 方法的类型编码。方法、SDK、系统或资源不受支持时，命令会失败。使用旧 SDK 构建时，其他 Apple adapter 操作仍可使用，此命令在开始推理前拒绝执行。

这是实验性的待处理元数据命令。`needsPreprocessing` 和 `ready` 表示 Apple 能识别元数据，不能保证后续的 Apple 预处理支持每一种输入。在 macOS 27.0.1 上，移除电影数据的 1080p/4K HEVC 相机输入及独立编码的 1080p H.264 输入，已完成公共预处理、渲染和编辑重载。相同 1920×1080、30 fps 短片的 H.264、HEVC 和 ProRes Standard 对照也均通过，公共处理路径不要求输入必须使用 HEVC。独立的 640×360 H.264 输入能达到待处理状态，但预处理返回 `CNCinematicErrorDomain` 错误码 5（`unsupported`）。修改色彩标签或虚拟传感器元数据没有消除原生尺寸检查的拒绝。命令保留压缩媒体，不通过缩放或转码绕过这个限制。其他尺寸、编码格式、系统及 iPhone 相册处理需要各自的消费者验证。

`convert` 和 `batch` 都会自动选择焦距输入。显式 `--focal-length-35mm MM` 优先；否则 Rust 通过 ExifTool 读取固定的已记录 35mm 等效焦距，没有已知值时运行已安装的 GeoCalib helper。不再默认填入 24 mm。只有主资源上明确的等效焦距标签合格：EXIF/XMP `FocalLengthIn35mmFormat` 和 QuickTime `camera.focal_length.35mm_equivalent`。物理 `FocalLength`、镜头名称、合成字段和嵌入缩略图信息不合格。EXIF 零值表示未知。多个已知值须在 0.001 mm 内一致；格式错误或冲突会失败，需显式指定焦距。

焦距必须有限且处于 1 至 300 mm。`--aperture F` 仍是独立拍摄假设，默认为 2.8，允许有限的 1 至 64。显式传入任一参数都需要 `--apple-cinematic`。记录的等效焦距可能没有计入防抖裁切；模型估计的是有效视角等效值，也可能出错。两者都不能恢复拍摄光圈或对焦距离。

GeoCalib 独立估计五个采样帧，再拟合共享固定视角相机。Rust 在原生生成前检查来源、源哈希、不确定度和跨帧一致性。依赖缺失、模型错误或估计被拒绝时会失败，不发布 MOV，也不代入猜测焦距。首版模型路径支持正方形像素的固定视角 SDR 视频，不估计变焦时间线。批量处理中每个输入独立解析。仅供 `convert` 的 `--cinematic-camera-estimate camera.json` 仍是高于自动元数据/模型选择的显式源绑定观测；显式焦距优先级更高，且此时不读取 JSON。

安装 ExifTool；需要模型回退时，再安装 FFmpeg/FFprobe 和 [GeoCalib helper](../model_adapters/geocalib/README.md)。Rust 从 Apple adapter 同目录或 `PATH` 查找 helper；`XDREMUX_EXIFTOOL` 和 `XDREMUX_GEOCALIB` 可指定可执行文件路径。runtime 将 ExifTool 限时 30 秒、GeoCalib 限时 600 秒、stdout/JSON 限为 1 MiB、诊断限为 64 KiB。结束或失败时终止所持进程组并回收子进程。正常命令不需要手工生成 JSON。`convert` 输出的 `focal-length-source` 为 `explicit`、`recorded-metadata`、`geocalib` 或 `camera-estimate`。

## 处理所需的最低信息

推理前会先解码视频。即使编码受支持，普通视频仍需要有效的 Cinematic 封装。在已测试的公共路径中，封装包含全局 `com.apple.quicktime.cinematic-video` 数据、具有有效检测 ID 和对焦决策的逐帧 cinematography，以及具有相机几何假设的逐帧 rendering。视频和元数据必须描述相同的可见时间线。`needsPreprocessing` 不要求已有栅格视差轨道。

诊断消融通过重命名元数据键，保持媒体字节和容器偏移不变。停用 cinematography 键后，能力变为 `none`。停用全局数据后，资产仍是待处理状态，但资源未就绪。停用逐帧 rendering 后，待处理资产的资源虽就绪，处理仍失败。这证明待处理分类本身不能验证所有处理输入。

在测试的 1080p H.264 短片上，同时停用可识别的 intent、`metadata.1`、aperture/disparity 标量和 scene-illuminance 键后，公共预处理、渲染和编辑重载仍通过。保留的 rendering/script 数据仍携带各自的数值。只有 AF 轨道及一个起始决策的脚本也通过。这些测试不会从生产输出中删除对应字段，也没有建立 Photos 所需的最小 schema。编码本身不能恢复物理相机自动对焦、原始拍摄标定或缺失深度。

## 原生生成路径

1. 使用 AVFoundation 解码选定的彩色视频轨道。
2. 使用 Apple 的 `PTMonocularDisparityProvider`，按假设焦距从解码帧推理临时视差。使用 Vision 检测人脸，没有人脸时同时检测上半身和人体姿态关节点。双耳关节点置信度均至少为 0.3 时，使用其中点和间距建立头部种子：宽度为水平耳距的 1.4 倍，归一化宽度允许范围为 0.01 到 0.5，按已定向图像宽高比将显示像素中的正方形区域转换到归一化坐标。优先保留最多两个头部种子，否则使用上半身种子。将视频显示方向传给 Vision，再把检测矩形映射回编码坐标。没有人体候选时，先使用 Vision 动物识别，标签置信度至少为 0.5。将猫/狗的头部关键点匹配到识别到的身体：鼻子及至少一只眼睛的置信度需达到 0.3，至少有三个可靠头部点，且头部框至少 60% 位于对应身体框内；否则使用身体候选。之后才在没有候选时，选择置信度最高的对象显著性区域。置信度必须至少为 0.3，区域面积必须占画面的 0.5% 到 60%。对象种子标记为 `unknown`，不表示检测到了人物。
3. 使用公共 `CNObjectTracker` 延续最多两个主体。在检测器短暂漏检时，只要跟踪边界和置信度仍有效，就保留轨道。将检测结果与这些主体匹配，为新主体分配新的轨道和分组。各轨道的检测类型保持不变；改变检测类型并重新开始跟踪时，在同一人物分组内建立新轨道。使用匹配的头部检测区域校正追踪漂移。保留仍有效的已选主体；检测到的头部矩形面积超过当前选中矩形面积的 1/0.6 倍时，允许接管焦点。优先选择人物而非非人物种子。没有主体时，选择画面中心的自动对焦区域。自动对焦轨道独立于主体轨道。使用 Apple 原生拍摄轨道/对焦分配器命名空间，保留公共编辑命名空间。每次主体切换时生成基础决策。使用 `PTCinematographyPostcaptureRefinement` 生成并序列化脚本。转焦期间保持细化后的焦点检测对象与帧头一致，基础选择单独存储。
4. 写入新的全局 Cinematic 元数据，并为每个解码帧写入一组逐帧 Cinematic 元数据。未知的拍摄对焦/视差使用原生待处理标记 `-1`。可见裁切设为 `(1,1)`，使 Apple 的解析器保留逐帧假设焦距。虚拟传感器区域设为编码图像尺寸。脚本和逐帧渲染数据都写入光圈。
5. 将压缩视频及全部受支持的音轨复制到 MOV，保留视频变换和轨道时间基。元数据生成后移除临时推理缓冲，不写入栅格视差轨道。
6. 使用公共 `CNAssetInfo` 加载新 MOV。只有确认 `needsPreprocessing`、资源状态 `ready` 和 `isPreprocessed == false` 后，runtime 才发布文件。

动物匹配同时使用头部与身体框，避免将不同物种或显著区域混为同一主体。只要公共跟踪器仍有效，短暂缺少姿态点不会立即把头部降为身体。相同动物部位返回时复用该主体组中的轨道 ID，保持逐轨道类型稳定；步骤 3 中的人体类型重建仍分配新轨道。

全局渲染数据使用兼容的虚拟拍摄配置和渲染版本 5。硬件/传感器字段、虚拟传感器区域及由亮度推导的场景照度代理值都是合成值，不能恢复原相机标定、防抖裁切或原始对焦选择。输出不会复制任何原生样本的 Cinematic 载荷或视差轨道。

生成器保持选定的焦距不变。可选的外部观测从采样图像估计有效视场；原生生成器不推理光学变焦，也不恢复物理相机标定。原始 Cinematic 几何、视差图和对焦决策仅作为诊断参考。主体连续性使用独立的选择规则，不复现 Apple 封闭的自动选焦算法，也不保证与原相机的对焦决策一致。

Rust CLI 仍是产品入口。Swift/Objective-C adapter 执行 Apple 框架操作。输出是独立文件。

## 验证

私有样本可以保留在 Git 外。在本工作树中执行以下命令，使用新的结果目录和新的或已有的构建目录。结果目录必须尚不存在。

```bash
python3 scripts/apple/verify_cinematic_envelope.py \
  --input /absolute/path/native.MOV \
  --run-dir /absolute/path/new-results \
  --swift-build-dir /absolute/path/swift-build \
  --rust-build-dir /absolute/path/rust-build \
  --synthetic --compare-render --verify-batch
```

`--verify-batch` 还会将受支持的普通副本与合成输入通过目录发现和并行 `batch --apple-cinematic` 处理，检查 MOV 结果类型、媒体包一致性以及每个输出的公共预处理、渲染与编辑重载。`--no-previews` 省略可重新生成的 PNG，保留数值证据。`--automatic-reference` 仅在内存中删除原生用户决策，用基础脚本焦点进行渲染，并记录参考模式。重复 `--input` 可加入更多原生样本。runner 构建当前 CLI 和 adapter，移除各原生样本的全局元数据以及所有非视频/非音频轨道。公共 `CNAssetInfo` 必须将普通副本判定为 `none`。随后 runner 调用 CLI，生成独立的待处理元数据封装。

公共消费者检查会验证脚本、拍摄 ID 命名空间、选定检测对象的轨道能否解析、逐轨道检测类型不变、有限的逐帧焦点、帧数、时间戳、光圈、逐帧键、渲染帧属性、虚拟传感器几何、逐帧焦距范围以及待处理资源状态。检查会加入两个公共固定焦点轨道，要求分配的 ID 与拍摄 ID 不冲突。待处理资源必须先预处理，才能编辑。检查记录基础决策数量和焦距时间线。FFprobe 比较实际编码视频/音频包的哈希、时间戳和音频裁切数据。AVAssetReader 的音频缓冲分组不能用于证明媒体包相同。runner 在各操作后复查源文件哈希。`--synthetic` 加入没有原始 Cinematic 数据及可检测人脸的独立 1080p 和 360p H.264 输入。使用 `--compare-render` 时，1080p 输入必须通过完整原生处理检查。360p 输入单独探测兼容性：只有 Apple 明确的预处理 `unsupported` 错误会记录为限制，其他错误会使验证失败。不受支持的兼容性样本不计入渲染或编辑证据；后续系统若支持该样本，则继续执行完整渲染与编辑检查。

将待处理 MOV 导入 Photos 测试图库。开启电影效果，等待 Photos 处理资源，调整光圈与焦点，然后保存并重新打开编辑，验证渲染效果确实变化。公共预处理和脚本重载测试不能替代此消费者检查。普通输入没有电影模式音频元数据；命令保留原始音频样本，但不生成 Apple 的电影模式音频混音效果。

## 无头电影效果对照

`--compare-render` 分别调用公共 `CNAssetInfo` 预处理原生资产和重新生成的资产，两者都必须变成 `renderable`。随后使用公共 `CNRenderingSession` 的 export 质量，在共同时间范围的 25%、50% 和 75% 位置分别以 f/2 和 f/16 渲染。两者使用相同解码色彩、时间戳、10 位输入格式、输出几何和单位显示变换。

预处理后，工具还会为生成资产加入两个公共自定义检测轨道和对焦决策，修改光圈，序列化 `CNScript.Changes` 并重新加载。检查轨道 ID 无冲突、所有保存的检测对象及时间戳、有限的逐帧焦点、光圈，以及两个编辑决策时刻的焦点。工具记录其间焦点曲线的变化：公共加载器会重新计算转焦，包括原生相机对照，因此不要求这些转焦与即时内存曲线完全相同。合成输入没有原生参考，同样执行预处理、export 渲染和编辑重载检查。平坦场景不要求可测量的虚化变化。单独检查生成文件时，可使用渲染器的 `--verify-generated GENERATED.mov NEW_OUTPUT_DIRECTORY`。

测量使用完整分辨率的 `RGBA16Float` 纹理 RGB 数值。原生 RGBA 输出保留源传递函数。HLG 输入对应 HLG 编码的 BT.2020 RGB，并非线性 sRGB。工具还将渲染图像转换为扩展线性 BT.2020，单独记录线性渲染与效果差异。两类测量都不包含 alpha。报告还导出预处理后整段脚本的检测 ID、组、类型、矩形、焦点、基础决策时刻与转焦范围；不同资产的原始 ID 不能直接当作共同主体身份比较。原生与推理深度栅格尺寸不同时，工具分别验证各栅格，并将深度差异标为不可直接比较；仍以不变的彩色视频分辨率分别原生渲染。工具计算解码色彩平面的哈希，并要求输入完全相同。报告记录像素完全相同与否、平均绝对误差、均方根误差、峰值信噪比，以及绝对差大于 0.001 的分量比例，同时记录各对焦矩形。工具比较推理视差及 f/2 与 f/16 的效果，分别测量完整画面和原生对焦矩形。效果余弦相似度用于判断虚化变化是否发生在相近区域。重复渲染原生资产用于测量确定性。

将生成版本设为原生对焦数值的渲染仅用作诊断对照。视差的缩放和偏移没有定义，相同的数值不能证明两份不同视差图采用相同的物理对焦平面。

比较新旧两轮验证结果时，使用：

```bash
python3 scripts/diagnostics/compare_cinematic_alignment.py \
  --baseline /absolute/path/baseline/report.json \
  --current /absolute/path/current/report.json \
  --output /absolute/path/current/alignment.json \
  --require-improvement SAMPLE_NAME --minimum-reduction 0.25
```

工具要求当前报告对应 `HEAD`，检查源文件哈希、普通输入完整性、媒体包一致性、解码色彩一致性、采样时间、参考焦点模式、渲染设置、原生渲染可重复性，以及参考效果能量是否不变。指定样本的平均 f/2 渲染误差和平均相对效果误差都必须下降。报告也会列出其他全部样本，包括退步的样本。该检查不会把测得的改进变成效果等同保证。

此前的报告将未经转换的纹理数值错误标注为 `raw linear RGB`。只有解码色彩哈希相同且原生参考效果能量不变时，才允许把这些编码 RGB 测量与当前的编码 RGB 测量比较。比较工具会记录标注修正。此前将 HLG 纹理解释为线性 sRGB 的 PNG 预览不能作为有效视觉参考。

PNG 是 SDR 预览。Core Image 使用解码缓冲的色彩空间、内容亮度余量和平均亮度。`CISystemToneMap` 以显示亮度余量 1 映射到标准动态范围。工具将源视频的显示变换从视频坐标转换到 Core Image 坐标，写入含 sRGB 配置文件的 8 位 PNG。这些预览不是浮点测量数据，也不证明与 Photos 的 Dolby Vision 动态修剪播放相同。测试预处理副本引用不变的输入彩色轨道，以避免重复存储，因此这些测试副本不能独立搬移。产品 `pending.mov` 仍是独立文件。

使用验证 runner 构建的渲染器运行 HDR 色彩回归：

```bash
/absolute/path/new-results/cinematic-render-comparison \
  --verify-color /absolute/path/new-color-results
```

已知 HLG 色块必须与经过色彩管理的参考一致。此前的线性 sRGB 解释必须无法通过负对照。SDR PNG 必须包含 sRGB 配置文件，并在系统色调映射后保留可区分的高光层次。不对称角标在 0、90、180、270 度下必须与独立的 Core Image 方向参考一致。

对照执行完成不代表效果相同。应结合 `all_compared_frames_identical` 和数值差异评估一致性。缺失的相机标定、光学几何或原始对焦决策，会让能够正常渲染的两份资产产生差异。本测试不证明 Apple Photos 保存/重开行为、设备相册行为，或未抽样帧的相同性。

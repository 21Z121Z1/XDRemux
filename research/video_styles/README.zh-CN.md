# 视频 Smart Style 载体实验

[English](README.md) | 简体中文

这个实验比较四种承载视频 Smart Style 元数据的候选 MOV 表达。**它仅用于研究。** 实验结果不能确立 XDRemux 的产品支持，也不能证明 Apple Photos 支持可逆的视频风格编辑。

Rust 负责产品策略和编排。本目录中的独立 Swift 程序只执行实验所需的公开 AVFoundation 操作。

## 候选定义

四种候选都写入六个静态 `com.apple.quicktime.smartstyle.*` 字段。其中三种还写入带盒装格式描述的时序元数据轨道，载荷为二进制属性列表。

| 变体 | 时序元数据 |
| --- | --- |
| `static` | 无；作为不含时序轨道的对照。 |
| `lower` | 使用首字母小写的完整字段名。 |
| `compact` | 使用缩短的字段名。 |
| `upper` | 使用首字母大写的完整字段名。 |

[`CarrierProbe.swift`](CarrierProbe.swift) 在同一个构造器中定义所有变体。常量包括版本/cast 为 1、强度为 1、tone/color 为 0、bypass 为 false；包含 reversible 字段时，该值为 true。这些值都是合成假设，不是恢复出的场景事实，也不是经过验证的原生格式。

## 运行实验

需要 macOS、Xcode 命令行工具和 `ffprobe`。合成冒烟测试还需要 `ffmpeg`。从仓库根目录运行：

```sh
swiftc -swift-version 5 -parse-as-library research/video_styles/CarrierProbe.swift -o /tmp/CarrierProbe
python -m research.video_styles.probe input.mov candidate.mov --helper /tmp/CarrierProbe --variant lower
python -m research.video_styles.smoke --helper /tmp/CarrierProbe
```

用 Python 包装器发布经过验证的候选。单独运行 Swift 构造器只会写入私有输出路径，不执行完整的发布检查。

目标路径必须尚不存在，父目录必须已存在。只处理你有权处理的媒体。包装器不上传媒体。

| 限制 | 数值 |
| --- | --- |
| 输入文件大小 | 最多 2 GiB |
| 子进程超时 | 120 秒 |
| Swift 读写循环超时 | 100 秒 |
| 时间戳读取器接受的视频样本数 | 最多 100,000 个 |

这些限制用于约束诊断工作量，不是产品性能指标。

## 发布前检查

[`probe.py`](probe.py) 在目标目录内的私有临时文件夹中构造候选。发布前必须通过以下检查：

- 原生接口回读静态和时序元数据载荷。
- 每个音视频编码包的 SHA-256 一致，各轨道内的包顺序和数量一致。
- PTS、DTS 和时长的精确有理数值一致。
- 编码器配置、位深、色度、色彩信息和显示矩阵一致。
- `ffprobe` 报告的源标签一致，但下述容器和轨道封装字段除外。
- 源文件哈希保持不变。

标签比较排除容器的 brand/encoder 字段，以及轨道的 handler/vendor/encoder 字段。上述检查不能证明所有未解释的 MOV 数据盒均被保留。

包装器用原子硬链接发布完成的候选。如果其他写入者先创建了目标，本进程会拒绝覆盖。原子名称发布不保证文件在断电或文件系统故障后仍然持久保存。

实验接受一条视频轨道和可选音轨。章节、其他轨道类型、缺失或重叠的时间信息、变化的编码格式描述，以及已存在的 Smart Style 字段都会导致失败。数据不一致或文件系统不支持所需操作时也会明确失败。

工具直通传递编码媒体，不重新编码、不猜测帧时长、不替换输入，也不把原生错误或超时转成成功。

## 原生时间戳观察

以下结果来自已记录的 macOS 26.6.2（25G83）实验，不代表本次重新运行，也不能证明 iOS 27 Photos 的行为。

压缩格式 `AVAssetReader` 在轨道呈现终点输出了零长度 `EmptyMedia` 控制缓冲区。将标记直接交给 `AVAssetWriter` 后，最后一个 H.264 B 帧的包时长从 1/12 秒变为 1/4 秒。编码载荷和轨道呈现时长仍然相同。显式调用 `endSession` 也未修复这个变化。

仅消费终点处的空标记后，所测 H.264、HEVC10、音频和旋转矩阵组合中的包及时间信息均得到保留。其他控制缓冲区和真实样本，包括音频裁剪附件，仍然原样传递。轨道内部的空编辑会明确失败。完整编码包比较仍是发布的必要条件。

合成冒烟测试要求实际执行终点标记处理路径。这个检查仅覆盖所测公开框架的行为。

## 失败报告与回归测试

探测失败时，程序输出 JSON 报告并返回非零退出码。时间信息不一致的报告保留：

- 媒体类型、源文件和候选文件的轨道 ID、流索引。
- 包索引；若差异位于流级时间信息，则为 null。
- 原始时间戳刻度、时基和精确有理数值。

工具在清理前计算临时候选的哈希。数据校验失败后，不发布该候选，也不将它保留为产物。源文件回读结果区分未改变、已改变和证据不可用三种情况。不可用证据记为 null。

合成检查覆盖 H.264/HEVC10 × `static`/`lower`/`compact`/`upper`，共八项。即使前面的项目失败，也会记录全部项目。八项都必须通过，并且必须执行终点空标记路径。检查还包括拒绝覆盖、源文件不变和拒绝无效输入三个对照。

缺少前置条件不能算作拒绝无效输入的对照通过。CI 在成功和失败后都会上传 JSON 诊断，但上传成功不能覆盖测试失败。如果载体实验已成功发布，而后续对照失败，整体测试仍然失败。报告会如实记录此前的发布。

运行可移植回归：

```sh
python -m unittest tests.test_video_style_research tests.test_video_style_diagnostics -v
```

原生结构检查位于[研究工作流](../../.github/workflows/research.yml)。工作流生成带音轨和旋转信息的 H.264 与 HEVC 10-bit 视频。这些合成视频不是原生 Smart Style 正样本。研究 CI 通过不改变产品支持范围。

## 历史实验与已弃用方法

早期任务在源文件下载、LFS 处理、Swift 桥接或并发检查中出现过失败。旧校验器只读取第一个视频样本，无法证明完整媒体保留、元数据回读、Photos 准入或编辑、保存、重新打开的行为。

旧写入器先删除已有目标，再验证输入。缺少时长时，它还假定帧率为 30 fps。这两种方法均已停用。

已删除的固件分析工作流请求过 iOS 27 构建 `24A437`，探测目标包括：

- `PESupport.assetCanRenderStyles:`
- `PESupport.canPerformEditOnAsset:`
- `PISemanticStyleAutoCalculator.canRenderStylesOnComposition:`
- `PISemanticStyleAutoCalculator.isStylableFromImageProperties:error:`
- `PHAssetPhotosSmartStyleExtendedProperties.isCurrentlySmartStyleable`
- `PHAsset._setupSmartStyleFromFetchDictionary:`

这些名称表示探测目标，不表示成功观察。反汇编输出未提交，工作流也允许部分失败以成功状态退出。CameraUI 地址 `0x1bdd478fc` 和 `0x1bdf25490` 只适用于特定构建，不能用作可移植入口。本目录不恢复固件下载器、私有二进制或一次性固件分析工作流。

## 进入产品前的要求

进入产品前，需要可合法共享的原生正样本和对照样本，并记录操作系统与设备来源。每个样本都必须具备正确的元数据语义。重复写入静态值不能确立这些语义。

验证必须覆盖完整时长、音轨、色彩/HDR 和方向保留。在适用设备上，还必须覆盖 Photos 导入、独立参数编辑、保存、重新打开和撤销。当对照或负测试否定某个候选时，应淘汰该候选。

将原始相册端观察及失败记录与源文件、输出文件哈希一同保存。在上述要求通过前，无论 CI 状态如何，`photosEditingValidated` 都保持 false。

## 一手参考资料

- [AVFoundation 导出指南](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/AVFoundationPG/Articles/05_Export.html)：将输出设置设为 nil 可请求压缩数据直通。必须检查读写器的完成状态和错误。
- [元数据适配器](https://developer.apple.com/documentation/avfoundation/avassetwriterinputmetadataadaptor)：时序组需要明确的盒装格式描述。以安装的 SDK 和原生回读结果核对行为。
- [ffprobe](https://ffmpeg.org/ffprobe.html)：`-show_packets -show_streams -show_data_hash sha256` 提供编码包和编码器附加数据的校验值。精确有理数时间戳可避免基于舍入值的比较。

# 视频 Smart Style 载体假设

[English](README.md) | 简体中文

**仅用于研究。** 这里比较四种候选 MOV 表达，不是 XDRemux 支持的转换功能，
也不能证明 Apple Photos 支持可逆的视频摄影风格编辑。Rust 仍独占产品策略和编排；
此处的独立 Swift 程序只执行实验所需的公开 AVFoundation 操作。

## 保留的问题与观察

历史实验尝试了六个静态 `com.apple.quicktime.smartstyle.*` 字段，以及 boxed
时序元数据轨道中 binary plist 的 `lower`、`compact`、`upper` 三种拼写。
`static` 是不添加时序轨道的对照。版本/cast 为 1、强度为 1、tone/color 为 0、
bypass 为 false、reversible 为 true，均为**人工假设**，不是场景事实或已验证格式。
各拼写在同一个构造器中比较，不形成多个产品后端。

历史任务出现过下载/LFS 和 Swift 桥接、并发错误。旧校验器只读取第一帧，不能证明
完整音视频保留、元数据载荷回读、Photos 准入或编辑/保存/重开。旧写入器还会先删除
已有目标，再检查输入；缺少时长时猜测 30 fps。这些做法已废弃。

已删除的固件分析 workflow 请求 iOS 27 `24A437`，探测目标包括
`PESupport.assetCanRenderStyles:`、`PESupport.canPerformEditOnAsset:`、
`PISemanticStyleAutoCalculator.canRenderStylesOnComposition:`、
`PISemanticStyleAutoCalculator.isStylableFromImageProperties:error:`、
`PHAssetPhotosSmartStyleExtendedProperties.isCurrentlySmartStyleable` 和
`PHAsset._setupSmartStyleFromFetchDictionary:`。这些只是**探测目标**，不是成功结果：
反汇编产物未提交，且脚本容许失败后仍成功退出。CameraUI 的两个特定构建地址
`0x1bdd478fc`、`0x1bdf25490` 也不是可移植入口。这里不恢复固件下载器、私有二进制
或一次性的固件分析 Actions。

## 复现实验

在安装 Xcode 命令行工具和 `ffprobe` 的 macOS 上，从仓库根目录运行：

```sh
swiftc -swift-version 5 -parse-as-library research/video_styles/CarrierProbe.swift -o /tmp/CarrierProbe
python -m research.video_styles.probe input.mov candidate.mov --helper /tmp/CarrierProbe --variant lower
python -m research.video_styles.smoke --helper /tmp/CarrierProbe
```

只处理具有相应权限的媒体。包装器不上传照片或视频。CI 自行生成带音轨和旋转矩阵的
H.264、HEVC 10-bit 视频，不把它们当作原生摄影风格正样本。输入上限为 2 GiB，子进程
超时为 120 秒，时间戳读取器最多接受 100,000 个视频样本。这些限制用于约束诊断过程，
不是产品性能指标。

`probe.py` 要求目标尚不存在、父目录已存在，在同目录私有临时文件夹内构造候选。
原生静态/时序载荷回读通过后，还必须核对全部编码包 SHA-256、各轨道内的顺序和数量、
精确有理数 PTS/DTS/时长、codec 配置、位深/色度/色彩和显示矩阵。源文件由 ffprobe
识别的标签须保留，但容器 brand/encoder 与轨道 handler/vendor/encoder 等封装信息
不在比较范围内；不宣称保留所有未解释的 MOV atom。输入哈希必须未变。全部通过后才
以同文件系统硬链接原子发布；若其他写入者抢先创建目标，本进程拒绝覆盖。原子名称发布
不等于断电或文件系统故障后的持久性保证。

当前只支持一条视频和可选音轨。章节、其他源轨道、缺失或重叠时间信息、变化的 codec
描述、已存在的 Smart Style 字段、对比失败、不支持硬链接的文件系统均明确失败；不重新
编码、不静默降级、不猜测帧时长、不替换输入。应通过 Python 包装器发布，而不是直接
使用只面向私有临时输出的 Swift 构造器。原生错误和超时不会被转成成功。

## 原生时间戳回归

在 macOS 26.6.2（25G83）上，压缩格式 `AVAssetReader` 会在轨道呈现终点
输出零长度 `EmptyMedia` 控制缓冲区。直接交给 `AVAssetWriter` 后，即使编码
数据与轨道呈现时长未变，最后一个 H.264 B 帧的包时长仍从 1/12 秒变为
1/4 秒。显式调用 `endSession` 无法修复。仅消费终点处的空标记后，
H.264、HEVC10、音频和旋转矩阵中的所有包与时间戳均保持一致。
其他控制缓冲区和真实采样（含音频裁剪附件）仍原样传递。中间空编辑显式
失败，不会悄悄删除；完整包校验仍是发布门禁。合成 smoke 测试断言实际
经过此控制缓冲区路径。这只证明所测公开框架行为，不证明 iOS 27 Photos。

## 晋升门槛

结构可读不够。还需要带 OS/设备来源记录、可合法使用的原生正样本和对照；证明逐样本
元数据语义正确而非重复常量；核对完整时长、音轨、色彩/HDR 和方向；在支持设备上验证
Photos 导入、独立参数编辑、保存、重开和撤销。对照矩阵否定的假设应淘汰。原始消费者
观察和失败须绑定输入/输出哈希。通过前，无论 CI 如何，`photosEditingValidated` 都为 false。

## 设计依据与验证

[AVFoundation 导出指南](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/AVFoundationPG/Articles/05_Export.html)
说明 nil output settings 请求压缩数据直通，并须检查 reader/writer 完成状态和错误。
[元数据 adaptor](https://developer.apple.com/documentation/avfoundation/avassetwriterinputmetadataadaptor)
结合明确的 boxed 格式描述写入时序组；本机 SDK 和原生回读构成可执行依据。
[ffprobe](https://ffmpeg.org/ffprobe.html) 的 `-show_packets -show_streams -show_data_hash sha256`
提供全部编码包与 codec extradata 的校验值；有理数时间避免把舍入误差当作等价。

可移植回归：`python -m unittest Tests.test_video_style_research -v`。
原生结构检查位于 `.github/workflows/research.yml`，与产品验收分开，成功也不改变上述支持状态。

## 失败证据

probe 失败时输出 JSON receipt 并返回非零退出码。timing mismatch 保留媒体类型、
source/candidate track ID 和 stream index、packet index（stream timing 为 null）、
原始 tick/time base 和精确有理数。临时 candidate 在清理前计算 hash；parity 失败后
不会发布，也不会作为 artifact 保留。source hash/readback 区分未改变、改变和证据
不可用；不可用值为 null。

synthetic gate 即使遇到前序失败也记录 H.264/HEVC10 × static/lower/compact/upper
全部八项。八项结果、terminal empty-marker coverage、no-clobber/source immutability
和 malformed-input rejection 都是必需项。prerequisite 缺失不能让 malformed control
通过。CI 在成功或失败时均上传 JSON 诊断，但不会豁免失败退出码。carrier trial 成功
而后续 smoke control 失败时，整体仍失败，并如实记录此前已完成的 publication。

诊断回归：`python -m unittest Tests.test_video_style_research Tests.test_video_style_diagnostics -v`。

# LibLivePhoto 接入

[English](liblivephoto-integration.en.md)

LibLivePhoto 负责可移植的 Motion/Live Photo 文件格式契约。XDRemux 通过
其资源 facade 完成源检查、分类、主资源选择、精确时间和 pair 验证。
Cargo manifest 和 lockfile 用同一完整 Git revision 固定两个独立库依赖。
LibLivePhoto 可以在 XDRemux workspace 外构建。

`xdremux-format` 重新导出 `liblivephoto-format` 的 cursor、error、FourCC、
JPEG、原始 EXIF 和 BMFF 基础类型。产品 codec profile 检查仍在本地。
`xdremux-motion-photo` 保留产品几何策略、文件复制和原子 pair 发布。
旧名称通过不稳定的迁移 feature 重新导出。新格式操作应使用 `MotionPhoto`、
`Input`、资源和 `MediaTime`，不得在 consumer crate 增加另一套 parser。

runtime 保留已知的整数和 timescale 时间，不再把已知 XMP 时间舍入到视频帧。
源时间未知时，现有产品 fallback 选择一个位置，并在回执中标记为推断。
Apple 几何元数据接收显式调用者值。OPPO 矩阵和防抖策略仍归 XDRemux 所有。

JPEG 和 Gain Map 编码属于产品操作。回执报告精确时间、视频 payload 字节比较、
元数据改写、重编码需求、新 identity 以及未进入 pair 的资源。CLI 报告资源遗漏。
原片保持完整。产品 pair 转换不承诺完整 vendor 资源保留或无损。
需要这种保留时，使用独立库的 source-copy 或显式 sidecar 策略。

验证使用现有 parser/payload conformance 和 14 个固定原片的 CLI 转换门禁。
runtime 测试另行比较源时间与写入时间，并检查转换报告。
completion gate 将本地检查绑定到已提交 HEAD；GitHub 提供第二验证环境。

原厂 Gallery/Photos 验收是独立要求。read、write、fixture、CI 和 device 状态
不得合并成一个支持结论。完整支持与证据边界见英文 canonical 文档的链接。

公共 consumer 合约使用真实 ColorOS 16 双视频样本。默认 Apple 计划会阻止
无法映射的厂商资源；显式 sidecar 策略逐字节保留这些资源。输出 pair 经重新
读取后检查精确时间和编码媒体一致性，并拒绝截断 MOV。这些证据不确定第二视频
的语义，也不证明原厂相册支持。

接入保留当前产品 HDR 应用色彩空间标记、摄影风格与 Cinematic 路径。共享格式
解析归 LibLivePhoto；HDR 决策和编解码仍归 XDRemux。独立库转换示例和外部
workspace 检查可脱离产品栈执行 Android/Apple 往返。Apple 转 Android 的整数
微秒限制见独立库兼容性文档。

`xdremux-compat` 仍是不稳定的迁移边界。每个旧 re-export 应在调用方改用公共
资源或 Apple 基础 API、对应 conformance 门禁通过后移除。文件复制、原子发布和
产品几何策略保持本地所有权。完成这些调用方迁移前不移除整个兼容 feature。

独立库现从 `ftyp` 读取受支持主视频的 MIME。Android JPEG/HEIF 目录会保留 QuickTime MIME，视频字节保持不变。编码规范符合性及原厂相册验收仍需分别验证。

主视频即使已有呈现时刻，也必须通过视频轨与 sample table 验证。已知整数/时间尺度保持原值，只有缺失时刻才使用推断值。`convert` 和新转换的 `batch` 项均报告遗漏资源数量；复用 checkpoint 的项没有新转换报告，遗漏数量保持未知。SEF 回退中发现有效 HDR Gain Map 但没有映射关系时，转换会拒绝，不会静默输出 SDR。

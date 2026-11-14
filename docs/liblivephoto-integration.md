# LibLivePhoto 接入

[English](liblivephoto-integration.en.md) | 简体中文

LibLivePhoto 提供可移植的 Motion Photo 和 Live Photo 文件格式实现。XDRemux 通过资源接口完成检查、分类、主资源选择、精确呈现时间处理和配对验证。

两个 Git 依赖在 Cargo 清单和 `Cargo.lock` 中固定到同一个完整 Git 提交哈希。LibLivePhoto 可以在 XDRemux 工作区之外构建。依赖声明见 [xdremux-format](../crates/xdremux-format/Cargo.toml) 和 [xdremux-motion-photo](../crates/xdremux-motion-photo/Cargo.toml)。

## 职责划分

| 组件 | 契约 |
| --- | --- |
| `liblivephoto-format` | 提供游标、错误、FourCC、JPEG、原始 EXIF 和 BMFF 类型，由 `xdremux-format` 重新导出。 |
| LibLivePhoto | 负责源资源、文件格式解析、配对身份和精确呈现时间。 |
| `xdremux-motion-photo` | 负责产品几何策略、文件复制，以及保留已有目录项的配对发布。 |
| XDRemux 编解码器和运行时 | 负责 HDR 策略、编码配置、图像编码、转换报告和验证后的发布。 |

新增格式操作使用 `MotionPhoto`、`Input`、资源和 `MediaTime`。调用方 crate 不得再引入独立的格式解析器。

接入后，产品的 HDR 应用色彩空间标记、摄影风格和电影效果处理保持不变。共享格式解析归 LibLivePhoto，HDR 决策和编解码操作归 XDRemux。

## 复制精确载荷范围

本地 `copy_payload_range` 和 `copy_payload_range_with_options` 函数先在目标旁暂存指定字节。完整复制并同步暂存文件后，才替换目标。复制失败时，输入和已有目标均保持不变。

如果目标目录项参与输入路径解析，函数会拒绝操作，包括该目录项的别名。独立硬链接或安全的目标符号链接可以作为目录项被替换，其指向的对象不变。

别名检查可能需要目录读取权限。检查失败时，两个目录项均保持完整。调用方必须在复制期间保持路径解析稳定。该契约不保证断电持久性。

实现及回归测试见 [payload.rs](../crates/xdremux-motion-photo/src/payload.rs)。

## 发布配对文件

### 公共辅助函数

`publish_live_photo_pair` 在成功和失败时都保留调用方拥有的暂存文件。暂存文件可以位于同一文件系统的子目录中。函数从不替换或删除最终目录项。

第二个成员发布失败时，`PairPublicationError::partial_output()` 返回已观察到的不完整输出路径。其他写入者随后可能修改该目录项，因此报告路径不构成删除权限。

该函数不接收源媒体或期望的资产身份，因此不能执行运行时的完整产品校验。`reconcile_live_photo_pair` 保留来源未知的不完整配对、无效配对和全部附属文件。实现见 [publication.rs](../crates/xdremux-motion-photo/src/publication.rs)。

### 运行时发布器

运行时的私有发布器只接收已通过全部必要检查的自有媒体字节：

- 严格的配对解析和验证。
- 预期的资产标识符。
- 精确呈现时间。
- 源视频与输出视频的压缩媒体载荷相同。

发布器负责暂存和文件同步。它通过有界读取，将暂存文件和最终文件与不可变媒体缓冲区比较。第二个成员发布失败或最终字节回读失败时，发布器保留暂存目录，并记录已观察到的最终目录项。

它尝试从已校验的缓冲区写入独立的 `recovery.heic` 和 `recovery.mov`，然后同步并回读这两个文件。只有这些操作成功后，错误信息才确认完整的独立恢复文件对已保留。否则，错误信息报告 `recovery-unverified`。原来的 `capture.*` 文件可能与受损的最终文件共享 inode。这些观察不构成删除权限。

调用方必须保持路径解析稳定。检查无法阻止同一用户的其他写入者在回读后修改文件。[发布契约](architecture.md#发布)说明文件系统要求和两文件事务的限制。

## 呈现时间与几何

即使源文件已经提供呈现时间，运行时也会验证主视频的视频轨和样本表。已知时间以整数和时间尺度表示，不会把已知 XMP 时间戳舍入到视频帧。

源文件缺少呈现时间时，产品策略选择回退位置，并在转换回执中标记为推断值。

Apple 几何元数据接收调用方明确提供的值。OPPO 矩阵和防抖策略仍由 XDRemux 决定，不是格式库的默认行为。

格式库从 `ftyp` 推导受支持主视频的 MIME 类型。Android JPEG/HEIF 目录输出保留 QuickTime MIME，视频字节不变。编码规范符合性与原厂相册验收需要分别检查。

## HDR 与资源保留

JPEG 和 Gain Map 编码仍属于产品操作。SEF 回退发现有效 HDR 数据、但没有对应的 Gain Map 资源映射时，转换失败，不会静默生成 SDR 输出。

Live Photo 回执记录精确呈现时间、压缩视频比较、元数据修改、重编码需求、新资产身份和遗漏资源。`convert` 与新转换的 `batch` 项均报告遗漏情况。复用检查点的项没有新的转换报告，因此遗漏数量保持未知。

原始输入保持完整。产品转换为 Apple 配对文件时，不承诺保留全部厂商资源，也不承诺结果无损。需要完整保留时，应使用格式库的源文件复制模式或显式附属文件策略。

## 可选 PVT 容器

PVT 是运行时对已验证 Apple 配对文件的封装。运行时写入真实目录，保持媒体字节不变，附加有大小限制的 XML plist，并排他发布整个目录。

PVT 不增加另一套 MOV/HEIF 解析器、资产标识符、编码操作或重封装。媒体解析、配对身份和精确呈现时间仍由 LibLivePhoto 负责。默认输出仍是配对文件。

名称冲突和恢复规则见 [CLI 封装说明](cli.md#pvt-目录输出)及[发布契约](architecture.md#发布)。包结构有效不能证明 Photos 导入或动态墙纸支持。

## Google Motion Photo 导出

`convert` 命令可选择 `google-jpeg` 或 `google-heic`。运行时直接交由 LibLivePhoto 完成格式解析、兼容性规划、XMP 改写、`mpvd` 封装和严格输出验证，不实现第二套 Motion Photo 写入器。新目标独立于现有 Apple 文件对/PVT 转换器，不改变其默认行为。

默认 `WritePolicy` 拒绝无法映射的资源。规划阶段拒绝静态图像格式转码和不能精确表示为整数微秒的时间戳。执行后，运行时对单文件严格重解析，对比源视频字节与呈现时间，并要求静态图像载荷不变的验证证据。同步写入临时文件后，通过排他、无覆盖操作发布。此机制保护已有目标文件，但不承诺断电后的完整持久性，也不代表已通过 Android 相册实机验证。

新增 Apple HEIC+MOV 输入路径，使用 `Input::ApplePair` 核验配对身份，不根据同名文件推断关联，再输出 Android HEIC。视频兼容策略仅保留 `preserve`：运行时要求视频容器逐字节一致，不转码、不改写品牌。资源审计逐字节比较全部辅助图像和 Gain Map XMP 属性，拒绝遗漏或丢弃源资源，并可通过 CLI JSON 列出每项处置。依赖仍锁定在经过检查的 LibLivePhoto 提交 `9328a0502bb2318b05d255ed83652c7d7c7d3b4c`；这是固定提交，不代表发布了新的稳定版本。

## 验证与证据边界

现有解析器和载荷符合性检查覆盖调用方边界。真实样本 CLI 检查转换并验证全部 14 个固定 Motion Photo 输入。运行时测试还比较已知源时间与写入时间，并检查转换报告。

公共调用契约使用原始 ColorOS 16 双视频样本。默认 Apple 转换计划拒绝无法映射的厂商资源；显式附属文件策略逐字节保留这些资源。重新读取配对输出后，检查精确时间和编码媒体一致性，并拒绝截断视频。

这项测试不确定第二视频的语义，也不证明原厂相册支持。详见 [liblivephoto_interop.rs](../crates/xdremux-motion-photo/tests/liblivephoto_interop.rs) 和[真实样本检查脚本](../scripts/validation/check_rust_motion_photo_real_fixtures.sh)。

完成检查将本地结果绑定到已提交的 `HEAD`。GitHub 在独立环境中运行调用方构建和检查。读取、写入、样本、CI 和设备结果必须分别描述。

独立库的转换器和外部工作区检查可脱离 XDRemux 产品栈执行 Android/Apple 往返。XDRemux 另验证 Apple HEIC+MOV → Android HEIC → Apple HEIC+MOV → Android HEIC，核验编码视频载荷和 HEIF 辅助图像字节。Apple 转 Android 时的整数微秒限制见库的兼容性文档。原厂相册和 Photos 验收仍是独立要求。固定库版本提供[格式支持说明](https://github.com/21Z121Z1/LibLivePhoto/blob/9328a0502bb2318b05d255ed83652c7d7c7d3b4c/FORMAT_SUPPORT.md)与[厂商样本证据](https://github.com/21Z121Z1/LibLivePhoto/blob/9328a0502bb2318b05d255ed83652c7d7c7d3b4c/fixtures/vendor/README.zh-CN.md)。

批处理复用相同的 Google JPEG/HEIC 单文件目标，没有新增写入器。检查点版本 3 区分 Apple 与 Google 路径，并区分视频兼容策略；Google 记录绑定完整输出 SHA-256，而非 Apple 配对标识。恢复前核验输入来源及严格解析后的输出。详见 [CLI 批处理约定](cli.md#batch)和[实机验收矩阵](validation/android-motion-gallery-matrix.md)。缺少证据前不启用三星／厂商原生输出及 AVIF 产品目标。

## 兼容功能

`xdremux-compat` 为既有底层调用方提供不稳定的迁移导出，不属于稳定公共 API。

调用方改用公共资源或 Apple API，且对应的符合性检查通过后，才能移除相应导出。载荷复制、安全发布和产品几何策略继续保留在本仓库。所有调用方迁移完成前，不移除整个兼容功能。

# 技术实现索引

[English](architecture.en.md) | 简体中文

本文档索引 XDRemux 当前稳定的实现契约。Rust workspace 是唯一产品实现；Swift 只保留明确受限的 Apple primitive adapter。

普通使用见[项目 README](../README.md)，命令行为见 [CLI 参考](cli.md)。

## 当前架构

### Rust workspace

Rust `xdremux` CLI 和 workspace crate 负责全部产品语义、格式转换、分类、批处理、Motion Photo 路由、验证和发布。

当前职责包括：

- ProXDR metadata 解析；
- ISO/TS 21496-1 Gain Map 转换；
- HEIF 和 ISO-BMFF 解析与写入；
- 通过独立 LibLivePhoto facade 完成 Motion Photo 路由和资源提取；
- 源 metadata 和分类；
- 核心转换链路共享的输出验证；
- Apple Portrait / Photographic Styles 的跨平台 policy、manifest 和 transaction orchestration。

LibLivePhoto 提供可移植的 Motion/Live Photo 格式实现。XDRemux 保留 HDR、codec
选择、推断时间、几何和文件发布策略。见 [接入契约](liblivephoto-integration.md)。

### Apple capability boundary

完整 Apple 产品仅分发 arm64/macOS 27 或更新版本；App 与原生包使用同一构建目标，adapter 在报告 capability 前拒绝不支持的 host。跨平台 Rust 核心测试独立保留，原生 framework 与设备验收仍须对应真实环境。

`Sources/XDRemuxAppleAdapter/` 是 Rust runtime 调用的最小 Apple framework primitive adapter。

当前职责包括：

- 调用 ImageIO、Vision、CoreImage、VideoToolbox 等 Apple framework；
- 返回 Rust 定义协议中的 framework facts；
- 写入或探测 Rust 已规划的 Apple-specific resource。

原来的 Swift product target 已删除。Swift 侧只剩 `Sources/XDRemuxAppleAdapter/`，不得承载 product policy。

### CLI 层

`crates/xdremux-cli/` 负责唯一用户命令解析和路由。

CLI 会在普通 HDR 命令链路前自动路由支持的 Motion Photo 输入。

Motion Photo 和普通 HDR 使用不同的输出安全规则。见 [CLI 参考](cli.md)。

### Python 实现

`xdremux_py/` 只包含研究/训练工具。

它不参与 Rust runtime，不定义正式 conversion path，也不作为 CI canonical correctness source。

### 模型原语

`model_adapters/geocalib/` 是独立的本地推理组件。Rust runtime 通过 ExifTool 读取已记录的等效焦距，只有缺失时才调用模型。Python helper 返回绑定源视频的像素内参和不确定度。来源优先级、固定相机准入、等效焦距换算、原生生成和发布均由 Rust 持有。helper 不构成第二套转换器或用户 CLI。固定版本环境与权重需要单独安装，不上传媒体。

## 稳定媒体契约

### 标准 HDR

标准链路把厂商 HDR metadata 转换为 ISO/TS 21496-1 表示。

所选链路允许时，实现会尽量保留源压缩图像数据。

### Live Photo

默认 Live Photo 链路发布共享 asset identifier 的 HEIC/HEIF 照片和 MOV。强类型 `LivePhotoOutput` 可将同一已验证 pair 包装为真实 PVT 目录，不再次跨越 codec/remux 边界。Rust 负责有界包结构、验证及发布，App 只转交选择并展示计划容器。

转换器把解析得到的源封面时间映射为 Apple `still-image-time`。

正常 MOV writer 使用压缩视频/音频样本透传。链路要求时，validator 会比较源文件和输出的压缩样本。

源文件存在 Gain Map 且输出链路支持时，still writer 会保留 Gain Map。

### 发布

默认 pair 输出只有在两个最终成员都发布后才算转换成功。转换先在自己独占的 staging 目录写入并同步已验证的完整 pair，再通过原子的 no-clobber 硬链接先占用 MOV 名称、后发布静态照片。已有最终路径及无关文件、目录不会被替换或删除。协作 writer 共享 MOV 占用，同 stem 的 `.heic` 与 `.heif` 也不例外。

两个文件名不是系统原子事务。第二个链接失败时，转换返回带 partial MOV 路径的错误，并保留完整 staging pair 供显式恢复。可对 staging 内的 `capture.heic` 执行 `xdremux validate`；移动资源前先检查两个最终路径。reconcile 只检查目标，未知 partial 或无效 pair 均保留并拒绝；名称像旧临时文件并不能证明删除所有权。

发布要求同一文件系统支持硬链接。不支持时保留原始 I/O 错误并拒绝，不宣称 FAT/exFAT 支持。此协议无法阻止外部 writer 主动替换最终路径。staging 文件已同步不等于断电后目录项具有持久性保证。

PVT 在 operation-owned 私有目录写入已验证媒体和固定 XML 版本 plist，然后用平台排他
目录 rename 占用单一最终包路径，即使已有空目录也不替换。移动前解除旧 staging 路径的
删除所有权，不会清理后来出现在旧路径的条目；发布失败保留完整包并报告恢复路径。
macOS/Linux 后端缺少排他 rename 时拒绝；其他核心平台可读取/验证支持的包结构，
不使用覆盖式发布回退。不据此宣称目录断电持久性、Photos 或墙纸通过。

batch 复用要求 source provenance 和容器身份一致。来源未知的有效 pair 或包不能被
接受为无关输入的输出。App 没有包来源的持久 checkpoint，因此不会自动接管已有 PVT。

## 当前技术文档

- [Apple 功能文档](apple-features.md)
- [开发文档](development.md)
- [测试政策](quality/testing.md)
- [验证 runbook](validation/README.md)
- [Fixture 说明](../fixtures/README.md)

## 历史审计

[ISO 一致性审计，2026-05-11](history/iso-conformance-audit-20260511.md) 是历史记录。

它包含当时的路径和实现细节。不要把其中的旧路径当作当前架构参考。

保留历史审计中的测量结果。新的 conformance 研究取代旧结论时，应增加新的带日期审计。

当前技术文档遵循[技术写作规范](style-guide.md)。

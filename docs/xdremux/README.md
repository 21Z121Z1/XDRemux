# v1.4 技术实现索引

[English](README.en.md) | 简体中文

本目录索引已经发布的 XDRemux v1.4 Swift/Python 线的稳定实现契约。

系统级 ownership、架构层和 branch 职责见[系统架构](../architecture.md)。Rust 迁移见[迁移路线图](../roadmap.md)。不要把下面的 v1.4 目录结构当作未来架构规范。

普通使用见[项目 README](../../README.md)，命令行为见 [CLI 参考](../cli.md)。

## v1.4 实现结构

### `XDRemuxCore`

`XDRemuxCore` 在 v1.4 中负责不需要 Apple feature layer 的格式和转换逻辑。

当前 v1.4 职责包括：

- ProXDR metadata 解析；
- ISO/TS 21496-1 Gain Map 转换；
- HEIF 和 ISO-BMFF 解析与写入；
- Motion Photo 解析和资源提取；
- 源 metadata 和分类；
- 核心转换链路共享的输出验证。

### `XDRemuxAppleFeatures`

`XDRemuxAppleFeatures` 在 v1.4 中负责 Apple 特有转换和验证。

当前 v1.4 职责包括：

- Motion Photo → Apple Live Photo；
- Live Photo 静态照片和 MOV 写入；
- Live Photo timing 和 asset identity；
- Live Photo writer 使用的厂商几何 policy；
- 摄影风格；
- Apple 人像；
- Apple 特有 native helper 集成。

### CLI 层

`Sources/XDRemuxCLI/` 负责 v1.4 用户命令解析和路由。

CLI 会在普通 HDR 命令链路前自动路由支持的 Motion Photo 输入。

Motion Photo 和普通 HDR 使用不同的输出安全规则。见 [CLI 参考](../cli.md)。

### Python 实现

`xdremux_py/` 是独立的跨平台 v1.4 实现。

它支持标准 HDR 转换、Motion Photo → Live Photo 和分类，不实现摄影风格或 Apple 人像生成。

Rust transition 中，只有当前 contract 或独立 evidence 支持时，才把这些实现作为有边界的 behavioral reference。不要为了形式对称保留它们的 file/module 切分。

## 稳定媒体契约

即使 implementation owner 发生变化，这些契约仍然是 migration input。

### 标准 HDR

标准链路把厂商 HDR metadata 转换为 ISO/TS 21496-1 表示。

所选链路允许时，实现会尽量保留源压缩图像数据。

### Live Photo

正常 Live Photo 链路发布共享同一个 asset identifier 的 HEIC/HEIF 静态照片和 MOV。

转换器把解析得到的源封面时间映射为 Apple `still-image-time`。

正常 MOV writer 使用压缩视频/音频样本透传。链路要求时，validator 会比较源文件和输出的压缩样本。

源文件存在 Gain Map 且输出链路支持时，still writer 会保留 Gain Map。

### 发布

Live Photo 输出是资源对事务。一个资源最终发布失败时，不能把另一个资源单独视为成功结果。

batch 复用要求 source provenance。来源未知的有效 pair 不能被接受为无关输入的输出。

Publication、provenance、collision handling 和 crash recovery 都是 product correctness contract。Rust rewrite 必须通过显式第 5 层 ownership 保留它们，而不是继续把它们当作 CLI-specific behavior。

## 当前技术文档

- [系统架构](../architecture.md)
- [迁移路线图](../roadmap.md)
- [Apple 功能文档](../apple-features.md)
- [开发文档](../development.md)
- [测试政策](../quality/testing.md)
- [验证 runbook](../validation/README.md)
- [Fixture 说明](../../fixtures/README.md)

## 历史审计

[ISO 一致性审计，2026-05-11](iso-conformance-audit-20260511.md) 是历史记录。

它包含当时的路径和实现细节。不要把其中的旧路径当作当前架构参考。

保留历史审计中的测量结果。新的 conformance 研究取代旧结论时，应增加新的带日期审计。

当前技术文档遵循[技术写作规范](../style-guide.md)。

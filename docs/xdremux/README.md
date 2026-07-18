# XDRemux 技术实现索引

本目录保存公开且相对稳定的 HDR、HEIF 和 ISO 容器实现说明。普通使用方式请从[项目首页](../../README.md)或 [CLI 参考](../cli.md)开始。

## 当前公开资料

- [ISO conformance audit](iso-conformance-audit-20260511.md)：ISO 21496-1、HEIF item/reference、tmap 和 Apple ImageIO 兼容审计。
- [验证说明](../validation/README.md)：结构、renderer、回归和设备证据的边界。
- [Apple 功能](../apple-features.md)：摄影风格和人像的用户能力、输入要求与验收范围。
- [开发文档](../development.md)：模块边界、helper、Swift Package API 和构建方式。

## 文档边界

公开技术文档描述当前实现约束和可重复验证方法。按日期记录的单样本实验、固件字段、逆向过程、未闭环假设和阶段性 UI 验收属于 `docs/research/` 或 `docs/experiments/`，不代表当前产品承诺。

当研究结论稳定并进入产品后，应先由代码和回归测试固化，再将面向用户或开发者的结论提炼到对应文档；不要把原始研究日志直接追加到 README。

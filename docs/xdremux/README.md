简体中文 | [English](README.en.md)

# XDRemux 技术实现索引

本目录收录 HDR、HEIF 与 ISO 容器行为的公开且相对稳定的文档。日常使用请先看[项目 README](../../README.md)或 [CLI 参考](../cli.md)。

## 当前公开文档

- [ISO 合规审计](iso-conformance-audit-20260511.md)：ISO 21496-1、HEIF item 与引用关系、tmap 行为，以及 Apple ImageIO 兼容性。
- [验证指南](../validation/README.md)：结构性、渲染器、回归与真机证据之间的边界。
- [Apple 功能](../apple-features.md)：Styles 与 Portrait 的用户能力、输入要求和验收范围。
- [开发指南](../development.md)：模块边界、helper、Swift Package API 与构建流程。

## 文档边界

公开技术文档描述当前的实现约束和可重复的验证方式。带日期的单样本实验、固件字段、逆向工作、未定假设和临时 UI 验收记录属于研究资料，保留在本仓库之外，不构成产品承诺。

研究结论稳定为产品行为后，先落到代码和回归测试里，再把面向用户或面向开发者的结论写进对应文档。不要把原始研究日志直接追加到 README。

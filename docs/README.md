# XDRemux 文档

[English](README.en.md) | 简体中文

安装和常用命令见[项目 README](../README.md)。

所有当前技术文档都遵循[技术写作规范](style-guide.md)。双语当前文档以英文版为 canonical source。

## 用户文档

- [CLI 参考](cli.md)：canonical Rust 命令契约、参数、输出位置和退出行为。
- [Apple 功能](apple-features.md)：摄影风格、Apple 人像以及当前平台能力迁移边界。
- [支持设备](supported-devices.md)：ProXDR 拍摄兼容性及其边界。

## 开发文档

- [开发与构建](development.md)：Rust 产品所有权、仓库架构、Apple adapter 边界和构建/测试流程。
- [测试政策](quality/testing.md)：修改必须提供的验证证据。
- [回归和真实样本验证](quality/evals.md)：可复用测试和 fixture gate。
- [输出政策](quality/logging.md)：stdout、stderr、JSON 输出和错误文本规则。
- [验证 runbook](validation/README.md)：completion gate 计划和证据类别。
- [测试套件说明](../Tests/README.md)：canonical 与迁移期测试套件。
- [Fixture 说明](../fixtures/README.md)：版本化 Motion Photo fixture 和文件身份规则。

## 技术实现

- [技术实现索引](xdremux/README.md)：稳定的实现契约。
- [ReverseKey1Ensemble 模型卡](../Models/ReverseKey1Ensemble.model-card.md)：可选研究模型契约。

## 历史记录

以下文件是证据记录。它们描述特定仓库状态或实验，不是当前产品规范。

- ISO 一致性审计，2026-05-11：[当前语言摘要](xdremux/iso-conformance-audit-20260511.summary.md) | [原始记录](xdremux/iso-conformance-audit-20260511.md)
- 编码质量和体积审计，2026-07-18：[当前语言摘要](validation/encoding-quality-pareto-20260718.summary.md) | [原始记录](validation/encoding-quality-pareto-20260718.md)
- 厂商 Live Photo 几何证据：[当前语言摘要](validation/vendor-live-photo-geometry.summary.md) | [原始记录](validation/vendor-live-photo-geometry.md)

当前产品行为以本页上方的当前文档为准。不要根据历史记录中的旧路径、旧测量值或旧实现说明推断当前保证。

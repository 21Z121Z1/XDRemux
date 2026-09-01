# XDRemux 文档

[English](README.en.md) | 简体中文

安装和常用命令见[项目 README](../README.md)。

所有当前技术文档都遵循[技术写作规范](style-guide.md)。双语当前文档以英文版为 canonical source。

## 系统导航

只读取当前任务真正需要的内容：

- [系统架构](architecture.md)：稳定抽象层、ownership、依赖规则和 source-of-truth 规则。
- [`agent-map.json`](agent-map.json)：机器可读 capability routing 和长生命周期 branch role。
- [迁移路线图](roadmap.md)：migration/research stage、promotion gate 和 branch retirement rule；它刻意不保存高频 Git 状态。
- [Agent 操作契约](../AGENTS.zh-CN.md)：低成本启动协议和 exact-HEAD 完成纪律。
- [执行计划契约](exec-plans/README.md)：跨 session/PR 工作的可恢复状态；小而边界清楚的修改不要使用。

较大的任务先实时推导 branch 状态：

```bash
python3 scripts/agent_context.py status
```

已知 capability 时直接路由，不需要全仓库扫描：

```bash
python3 scripts/agent_context.py capability engine.plan
```

编程语言本身不是架构边界；媒体语义、能力、产品契约和证据才是。

## 用户文档

- [CLI 参考](cli.md)：命令、参数、输出位置和退出行为。
- [Apple 功能](apple-features.md)：摄影风格、Apple 人像以及支持的组合。
- [支持设备](supported-devices.md)：ProXDR 拍摄兼容性及其边界。

## 开发文档

- [开发与构建](development.md)：v1.4 Package 产品、仓库结构、App 构建和集成方式。
- [测试政策](quality/testing.md)：修改必须提供的验证证据。
- [回归和真实样本验证](quality/evals.md)：可复用测试和 fixture gate。
- [输出政策](quality/logging.md)：stdout、stderr、JSON 输出和错误文本规则。
- [验证 runbook](validation/README.md)：evidence class、evidence role、completion-gate plan 和 receipt。
- [测试套件说明](../Tests/README.md)：Swift 和 Python 测试入口。
- [Fixture 说明](../fixtures/README.md)：版本化 Motion Photo fixture 和文件身份规则。

## 技术实现

- [技术实现索引](xdremux/README.md)：稳定的 v1.4 实现契约和产品链路细节。
- [ReverseKey1Ensemble 模型卡](../Models/ReverseKey1Ensemble.model-card.md)：已发布线中的可选研究模型契约。

Active research branch 可以包含尚未 promotion 到 released line 的额外 model card 和 research protocol。这些 branch-local 文档只对该分支研究事实具有权威性；进入产品行为必须通过 roadmap promotion gate。

## 历史记录

以下文件是证据记录。它们描述特定仓库状态或实验，不是当前产品规范。

- ISO 一致性审计，2026-05-11：[当前语言摘要](xdremux/iso-conformance-audit-20260511.summary.md) | [原始记录](xdremux/iso-conformance-audit-20260511.md)
- 编码质量和体积审计，2026-07-18：[当前语言摘要](validation/encoding-quality-pareto-20260718.summary.md) | [原始记录](validation/encoding-quality-pareto-20260718.md)
- 厂商 Live Photo 几何证据：[当前语言摘要](validation/vendor-live-photo-geometry.summary.md) | [原始记录](validation/vendor-live-photo-geometry.md)

当前产品行为以本页上方当前文档为准。不要根据历史记录中的旧路径、旧测量值、旧实现说明、PR 描述、completed plan 或 branch 名称推断当前保证。

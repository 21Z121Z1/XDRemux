# XDRemux Agent 指南

[English](AGENTS.md) | 简体中文

本指南规定 XDRemux 的修改与验证流程。Rust `xdremux` CLI、运行时、引擎和媒体 crate 构成唯一产品栈。Swift 提供 Apple 框架适配器，Python 提供研究和训练工具，以及 `model_adapters/` 下独立的模型原语。模型原语只返回观测。相机选择、准入和转换策略仍由 Rust 负责。

组件职责见[开发文档](docs/development.md)，证据选择见[测试政策](docs/quality/testing.md)。[验证操作指南](docs/validation/README.md)说明如何生成绑定具体提交的回执。[Apple 功能文档](docs/apple-features.md)规定平台边界。

## 保持产品契约

- 保留源媒体的真实信息和已经验证的资源关系。不得为使解析器接受文件而虚构 Apple 或厂商元数据。
- `xdremux-engine` 负责源信息、用户意图、能力要求、确定性策略和转换计划。
- `xdremux-runtime` 负责执行、验证顺序、文件系统操作，以及原子或可恢复的输出发布。
- Apple 适配器只执行必须调用 Apple 框架的操作，返回框架观察结果或操作结果。产品决策由 Rust 负责。
- 缺少必需的源数据、能力或验证时，停止操作。失败不得损坏输入或已有的发布输出。
- 区分结构检查、原生框架检查和真机测试。解析器检查通过不能证明 Photos 编辑或相册行为。
- 保持公开测试样本的字节和来源记录准确。私有媒体和只可用于真机验证的媒体不得进入公开 Git 或公开产物。

## 修改与验证

1. 从 Git 和 GitHub 读取当前分支、PR、修改路径和 CI 状态。不得把复制的状态文字当作当前证据。
2. 读取负责该行为的实现、附近测试和适用契约。
3. 完成最小且完整的修改。不得在 Swift、Python、脚本或研究代码中建立第二套产品策略。
4. 可行时，添加验证可观察行为的回归测试。只有源码结构本身属于契约时，才检查局部变量名或辅助函数调用。
5. 运行最小但完整的检查集。修改产品或验证基础设施时，完成证据必须绑定已提交的 `HEAD`。
6. 只报告证据能够证明的行为，明确列出仍需真机验证的部分。

先定稿作为权威来源的英文文档，再翻译为中文。两种语言必须保留相同的技术范围和限制。

## 使用项目技能

按适用条件使用 [pstack 流程](.agents/skills/pstack-workflow/SKILL.md)、[行为 TDD](.agents/skills/pstack-tdd/SKILL.md)、[工程原则](.agents/skills/pstack-engineering-principles/SKILL.md)、[审查](.agents/skills/pstack-review/SKILL.md)和[文字整理](.agents/skills/pstack-unslop/SKILL.md)。这些技能是固定来源版本的项目适配，不依赖 Cursor 运行环境。

实际 CLI 操作与证据选择见 [XDRemux 验证](.agents/skills/xdremux-verify/SKILL.md)。

| 契约 | 现有检查 |
| --- | --- |
| HDR 资源与变换保持正确关系。 | HEIF 与编解码器资源图测试、RGB16 回归，以及 `ci.yml` 中的 ImageIO 回读。 |
| 原生执行与编辑状态完整。 | Texture 回执和状态回归，以及原生消费者检查。 |
| 失败或并发发布保留媒体。 | 运行时批处理和 Live Photo 失败测试，以及 App 模型测试。 |
| 证据标明实际测试的源码与主机。 | 完成门禁将回执绑定到 `HEAD`；原生和真机结论需要对应环境。 |
| CI 使用可复用的工作流。 | 工作流测试将产品检查归入 `ci.yml`，研究检查归入 `research.yml`。 |

## 完成研究任务

区分产品实现、可长期保留的研究证据和诊断探针。每项实验必须以以下一种状态结束：

- **PROMOTE**。把已验证的契约纳入产品代码、测试或当前文档，删除已被取代的探针。
- **ARCHIVE**。保留具有长期价值的可复现证据、来源记录、结果和剩余缺口。
- **DISCARD**。结论已被吸收或实验没有长期价值时，删除探针。

分支名称、提交数量、结构产物和离线指标都不能证明产品已经支持某项功能。只有跨会话或 PR、依赖私有样本或真机，或需要经过多个阶段才能纳入产品的任务，才使用执行计划。

不得将聊天记录、思维链、会话日志、分支 SHA、ahead/behind 数量或当前工作流状态作为稳定的仓库知识保存。

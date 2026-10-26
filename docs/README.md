# 文档索引

[English](README.en.md) | 简体中文

构建 CLI 和首次转换见[项目 README](../README.md)。本索引分别列出当前产品说明、研究材料和历史证据。

## 使用 XDRemux

| 文档 | 内容 |
| --- | --- |
| [CLI 参考](cli.md) | 命令、选项、默认值、输出路径、错误和复用条件。 |
| [Apple 功能](apple-features.md) | 人像、摄影风格、纹理风格、源数据要求和原生功能限制。 |
| [电影效果视频](cinematic.md) | 待处理元数据、原生预处理和实验性视频支持。 |
| [支持设备](supported-devices.md) | 已报告的源文件兼容性及可用测试样本证据。 |
| [Live Photo 集成](liblivephoto-integration.md) | Motion Photo 转换、媒体保留、配对和 PVT 输出。 |

## 开发与验证

| 文档 | 内容 |
| --- | --- |
| [架构说明](architecture.md) | 实现模块和媒体约定。 |
| [开发与构建](development.md) | Rust 工作区、Apple 适配器、App 和打包。 |
| [测试政策](quality/testing.md) | 必需证据及各类测试的证明边界。 |
| [回归与样本检查](quality/evals.md) | 可复用检查和真实文件验证。 |
| [输出政策](quality/logging.md) | 标准输出、标准错误、JSON 和错误信息。 |
| [验证操作指南](validation/README.md) | 验证计划及绑定已提交源码状态的回执。 |
| [测试套件说明](../tests/README.md) | 测试职责和执行命令。 |
| [测试样本说明](../fixtures/README.md) | 版本化媒体和文件身份规则。 |
| [脚本索引](../scripts/README.md) | 构建、验证、性能和研究命令。 |
| [Agent 指南](../AGENTS.zh-CN.md) | 仓库规则与项目 pstack Skill。 |
| [技术写作规范](style-guide.md) | ASD-STE100、中文写作规则、术语及双语审阅。 |

当前双语文档以英文为原稿。两个版本必须保留相同的行为说明和限制。

## 阅读或复现研究

研究工具不定义公开产品约定。各份记录分别说明输入、测量或原生观察能够证明什么。

- [OPPO 风格研究](../research/oppo_styles/README.md)说明模型训练、推理和证据边界。
- [研究方法](../research/oppo_styles/methodology.md)定义数据选择、损失函数和评估方法。
- [ReverseKey1Ensemble 模型卡](../research/oppo_styles/models/ReverseKey1Ensemble.model-card.md)说明研究模型和二进制布局。
- [UniversalPhotographicStyleStateNet 模型卡](../research/oppo_styles/models/UniversalPhotographicStyleStateNet.model-card.md)说明联合研究预测器。
- [纹理风格研究](../research/texture_styles/README.md)记录纹理元数据和原生行为。
- [视频风格研究](../research/video_styles/README.zh-CN.md)检查视频元数据容器和消费端证据。
- [调色板研究](../research/palette/README.md)说明以源数据为依据的场景分类实验。

## 阅读历史证据

以下记录描述特定版本、实验或源码状态，其中的测量值不构成当前产品保证。

| 记录 | 阅读入口 |
| --- | --- |
| ISO 一致性审计，2026 年 5 月 11 日 | [摘要](history/iso-conformance-audit-20260511.summary.md)、[原始记录](history/iso-conformance-audit-20260511.md)。 |
| 编码质量与体积，2026 年 7 月 18 日 | [摘要](validation/encoding-quality-pareto-20260718.summary.md)、[原始记录](validation/encoding-quality-pareto-20260718.md)。 |
| 厂商 Live Photo 几何 | [摘要](validation/vendor-live-photo-geometry.summary.md)、[原始记录](validation/vendor-live-photo-geometry.md)。 |
| HDR Log 多样本检查，2026 年 10 月 14 日 | [验证记录](validation/20261014-hdr-log-multisample.md)。 |
| HDR Log 缩略图检查，2026 年 10 月 14 日 | [验证记录](validation/20261014-hdr-log-thumbnail.md)。 |
| 1.4 版本 | [发布记录](releases/v1.4.md)。 |

当前行为以当前参考文档为准。阅读旧结果时，保留历史路径、数值和结论的原有含义。

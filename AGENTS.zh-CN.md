# XDRemux Agent 指南

[English](AGENTS.md) | 简体中文

XDRemux 只有一套产品栈：Rust `xdremux` CLI、runtime、engine 和媒体 crate。Swift 只保留窄 Apple framework adapter。Python 只用于研究和训练。

所有权见[开发文档](docs/development.md)，证据选择见[测试政策](docs/quality/testing.md)，准确 `HEAD` 回执见[验证文档](docs/validation/README.md)，平台边界见 [Apple 功能](docs/apple-features.md)。

## 不变量

- 保留真实源事实和已验证的资源关系。不要只为通过 parser 而虚构 Apple 或厂商 metadata。
- `xdremux-engine` 持有源事实、用户意图、能力、确定性 policy 和转换计划。`xdremux-runtime` 持有执行、验证顺序、文件系统副作用以及原子或可恢复发布。
- Apple adapter 只执行必须使用 Apple framework 的操作，并返回事实或 primitive result。它不持有产品 policy。
- 缺少必需源数据、能力或验证时必须 fail closed。失败操作不得破坏现有输入或已发布输出。
- 结构、原生 framework 和真机证据属于不同层级。parser 通过不能证明 Apple Photos 编辑或相册行为。
- 公开 fixture 的字节和 provenance 必须保持准确。私有或只可真机验证的媒体不得进入公开 Git 或公开 artifact。

## 修改流程

1. 从 Git 和 GitHub 推导当前 branch、PR、changed path 和 CI 状态。不要信任 prose 中复制的状态。
2. 修改前读取 canonical owner、附近测试和适用 contract。
3. 只做最小且完整的修改。不要在 Swift、Python、脚本或研究代码中建立第二份产品 policy。
4. 可行时增加验证可观察行为的 regression。除非源码结构本身就是 contract，不要让测试绑定局部变量名或某次 helper 调用。
5. 运行最小但完整的证据集。产品代码或验证基础设施变化时，完成证据必须绑定已提交的 `HEAD`。
6. 只报告证据真正证明的行为，并明确写出依赖真机的缺口。

先直接编写并定稿英文 canonical 文档，再把相同技术范围和限制翻译成中文。

## 研究生命周期

产品实现、稳定研究证据和诊断 probe 是三类不同资产。每个实验都必须进入一种终态：

- **PROMOTE**：把已验证 contract 移入 canonical 代码、测试或当前文档，然后删除已被取代的 probe。
- **ARCHIVE**：只保留具有长期价值且可复现的证据、provenance、结果和剩余缺口。
- **DISCARD**：结论已经吸收或实验没有稳定价值时，删除 probe。

branch 名称、commit 数量、结构 artifact 或离线指标都不能让 feature 自动晋升。只有跨 session 或 PR、依赖私有 fixture 或真机，或者具有长期 promotion ladder 的任务才使用 execution plan。不要把聊天记录、思维链、session journal、branch SHA、ahead/behind 数量或当前 workflow 状态保存成稳定仓库知识。

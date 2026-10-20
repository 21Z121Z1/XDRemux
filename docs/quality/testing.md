# 测试政策

[English](testing.en.md) | 简体中文

只有验证证据与实际修改的行为匹配时，才能认为修改完成。

测试命令见 [tests/README.md](../../tests/README.md)。completion gate 计划见[验证 runbook](../validation/README.md)。

## 证据层级

| 层级 | 常用命令或来源 | 能证明什么 |
| --- | --- | --- |
| Unit 和 contract | `cargo test --workspace --locked` | Rust 产品、parser、格式、Apple feature policy 和 transaction 契约。 |
| 仓库 policy | `python3 -m unittest discover -s tests -t . -v` | 跨文件 policy、Python 行为、文档和架构契约。 |
| 真实 fixture | `fixtures/` 和 `scripts/validation/` | 版本化或提供的真实媒体行为。 |
| 原生框架 | macOS validation job | 被测试的 ImageIO、PhotoKit 或其他 Apple framework 行为。 |
| 真机 | 手工或记录的真实设备验证 | 依赖具体相册、Photos 版本、显示或设备的行为。 |
| Completion receipt | `scripts/ci/agent_completion_gate.py` | 选定检查在准确 commit 上通过。 |

静态 policy test 不是功能转换证据。

parser test 不是真机测试。

容器 parser 通过不能证明相册一定正确渲染结果。

## 最低验证要求

完整产品门禁面向 arm64/macOS 27：App、adapter、解包运行、原媒体保全、HDR/Styles 和性能检查均保留。Linux、Windows、macOS 的 Rust 核心测试继续执行，Intel 产品包不再属于分发范围。范围调整不把原生缺测、操作失败或未验设备行为变成通过。

使用最小但完整的证据集。

- 纯文档修改：文档 policy 和链接检查。
- CLI parser 或消息修改：对应 parser/output 回归。
- HDR 或容器修改：unit test 加真实转换或等价 functional fixture。
- Motion Photo 修改：parser/writer test 加适用的真实 Motion Photo fixture gate。
- Apple 功能修改：结构测试加适用的原生框架或真机证据。
- App 修改：App 构建和受影响的 model/UI test。
- 验证框架修改：completion gate 自测加一次真实 gate 运行。

实际可行时，每个源码 bug fix 都应该增加一个能在原缺陷上失败的回归断言。

## 公开 Motion Photo fixture

仓库的 `fixtures/` 目录包含版本化真实 Motion Photo fixture。

它们的准确字节属于测试契约。`fixtures/SHA256SUMS` 是文件身份 manifest。

严格 Rust CI gate 使用这些 fixture 测试多种 JPEG 和 HEIC/HEIF Motion Photo 布局。

不要再把所有真实样本描述为 private。部分旧 ProXDR、只可真机验证和 Apple feature 样本仍可能位于仓库之外或保持私有。

原生检查可以在 runner 内生成媒体和像素缓冲。Actions 自动上传只包含明确命名的诊断报告、
哈希和日志，不包含原片、转换后的人像、渲染对比图或像素字典。测试仍须完成媒体生成、
原生回读和失败断言；收窄上传边界，不跳过检查。需要设备验证时，在本地生成 Photos 导入文件。

## 依赖真机的结论

涉及 Apple Photos 编辑、OPPO 相册显示或其他设备 UI 的结论，需要来自对应环境的证据。

环境不可用时，只能把结论限制到实际验证的行为。

未测试的真机行为不能标记为通过。

## 文档也是可测试契约

面向用户的命令名、参数、默认值、输出安全规则和支持边界都属于产品契约。

代码改变这些契约时，同一个修改中必须同时更新英文和中文文档。

当前技术文档遵循[技术写作规范](../style-guide.md)。

## 研究回归证据

唯一可复用的 `research.yml` 工作流持有可移植研究测试、原生 carrier 回读、固定版本的 Photographic Styles tone 扫描，以及临时 system HEIF HDR 测量。各 job 使用只读权限并检出准确的 PR 或 main SHA。手动重放不接受替代源码或 secret。原生 tone 与 system HEIF job 使用标准 `xcode-27` runner，测量 artifact 保留一天。它们不提供 `exact-head`，也不能证明 Photos 编辑、滑块响应、保存重开或 Gallery 接受情况。

实验命令和晋升条件放在对应代码附近，不为每个假设新建工作流。产品方向回读在 `ci.yml` 中运行，与 Rust 产品测试使用同一准确 SHA，由 ImageIO 读取合成方向 HEIF 及其预期几何。

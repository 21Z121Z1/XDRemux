# 测试政策

[English](testing.en.md) | 简体中文

只有验证证据与实际修改的行为匹配时，才能认为修改完成。

测试命令见 [Tests/README.md](../../Tests/README.md)。completion gate 计划见[验证 runbook](../validation/README.md)。

## 证据层级

| 层级 | 常用命令或来源 | 能证明什么 |
| --- | --- | --- |
| Unit 和 contract | `cargo test --workspace --locked` | Rust 产品、parser、格式、Apple feature policy 和 transaction 契约。 |
| 仓库 policy | `python3 -m unittest discover -s Tests -v` | 跨文件 policy、Python 行为、文档和架构契约。 |
| 真实 fixture | `fixtures/` 和 `Tests/validation/` | 版本化或提供的真实媒体行为。 |
| 原生框架 | macOS validation job | 被测试的 ImageIO、PhotoKit 或其他 Apple framework 行为。 |
| 真机 | 手工或记录的真实设备验证 | 依赖具体相册、Photos 版本、显示或设备的行为。 |
| Completion receipt | `scripts/agent_completion_gate.py` | 选定检查在准确 commit 上通过。 |

静态 policy test 不是功能转换证据。

parser test 不是真机测试。

容器 parser 通过不能证明相册一定正确渲染结果。

## 最低验证要求

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

## 依赖真机的结论

涉及 Apple Photos 编辑、OPPO 相册显示或其他设备 UI 的结论，需要来自对应环境的证据。

环境不可用时，只能把结论限制到实际验证的行为。

未测试的真机行为不能标记为通过。

## 文档也是可测试契约

面向用户的命令名、参数、默认值、输出安全规则和支持边界都属于产品契约。

代码改变这些契约时，同一个修改中必须同时更新英文和中文文档。

当前技术文档遵循[技术写作规范](../style-guide.md)。

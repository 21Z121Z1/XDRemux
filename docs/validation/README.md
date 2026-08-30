# 验证 Runbook

[English](README.en.md) | 简体中文

本目录用于保存验证依据、验收条件和证据记录。

可执行测试放在 `Tests/` 或 `scripts/`。

## 证据类别

以下类别保持分离：

| 类别 | 示例 | 能证明什么 |
| --- | --- | --- |
| Static | 源码或文档 policy check | 文本、结构或架构契约。 |
| Regression | 针对已知缺陷的测试 | 指定缺陷在被测试条件下不再复现。 |
| Functional | 真实转换或等价媒体 fixture | 受影响产品链路可以在代表性数据上运行。 |
| Integration | framework 或 App 集成 | 多个组件在被测试环境中协同工作。 |
| Device | 真实相册、Photos、显示或设备测试 | 对应环境中的真机依赖行为。 |

更高层级证据可以包含低层检查，但不能改变无关检查本身能证明的范围。

## Completion gate

仓库 Agent 在目标修改提交后使用 `scripts/agent_completion_gate.py`。

示例：

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

plan 中每个命令使用参数数组。gate 不会添加隐式 shell 解析。

示例 plan：

```json
{
  "schema_version": 1,
  "scope": "Update current CLI documentation",
  "checks": [
    {
      "name": "documentation-policy",
      "kind": "static",
      "command": [
        "python3",
        "-m",
        "unittest",
        "Tests.test_public_documentation"
      ],
      "timeout_seconds": 120
    }
  ]
}
```

确实需要 shell 组合时，在命令数组中显式调用 shell。

## Receipt 契约

receipt 会绑定：

- 当前 `HEAD`；
- 选定 base commit；
- changed-file set；
- clean tracked worktree；
- 每个声明检查的退出状态和有限输出。

之后的 commit 或 tracked edit 会让 receipt 失效。

plan 中声明的所有检查都是必需项。

## 根据修改选择检查

默认使用有针对性的验证。

纯文档修改一般只需要文档一致性和链接检查。

转换核心修改一般需要针对性 regression 加 functional media evidence。

Motion Photo 修改应该在公开 fixture gate 能覆盖相关 parser、writer、timing 或 publication 行为时使用这些 gate。

涉及 Apple Photos 交互的结论，需要真正到达该行为的原生 framework 或 device evidence。

不要为了增加检查数量而运行昂贵且无关的矩阵。

## 公开和私有媒体

仓库的 `fixtures/` 下包含版本化真实 Motion Photo fixture。

其他 ProXDR、Apple feature 或只可真机验证的样本仍可以放在 Git 之外。

runner 能访问外部本地样本时，verification plan 可以使用绝对路径引用它。

## 历史验证记录

本目录中带日期的文件可能描述旧实现状态。

不要修改旧测量值来迎合当前实现。证据变化时，编写新的当前文档或新的带日期记录。

当前文档遵循[技术写作规范](../style-guide.md)。

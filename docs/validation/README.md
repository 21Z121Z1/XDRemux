# 运行验证并检查完成回执

[English](README.en.md) | 简体中文

本目录保存验证依据、验收条件和证据记录。可执行测试放在 `tests/` 或 `scripts/`。以下流程用于记录已提交修改的检查结果。

## 选择证据类别

| 类别 | 示例 | 能证明的范围 |
| --- | --- | --- |
| `static` | 源码或文档规则检查。 | 文本、结构或架构契约。 |
| `regression` | 针对已知缺陷的测试。 | 缺陷在被测条件下不再复现。 |
| `functional` | 真实转换或等价媒体样本。 | 受影响的产品流程能在代表性数据上运行。 |
| `integration` | 框架或 App 集成。 | 组件能在被测环境中共同工作。 |
| `device` | 相册、Photos、显示器或真机测试。 | 对应环境中的设备相关行为。 |

较高层级测试可以包含较低层级检查，但不会扩大无关证据的证明范围。例如，框架回读不能证明 Photos 编辑、保存和重新打开的交互。

## 准备并运行完成门禁

提交目标修改后，使用 [`scripts/ci/agent_completion_gate.py`](../../scripts/ci/agent_completion_gate.py)。已跟踪文件的工作树必须干净。计划中声明的每项检查都必须通过。

1. 根据修改的行为选择检查。
2. 编写验证计划，以参数数组表示每条命令。
3. 选择基准提交并运行门禁。
4. 验证生成的回执。

文档计划可以使用现有规则和链接检查：

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
        "tests.test_public_documentation",
        "tests.validation.test_repository_layout"
      ],
      "timeout_seconds": 120
    }
  ]
}
```

本例将计划保存为 `/tmp/xdremux-agent-verification.json`。在仓库根目录运行：

```bash
python3 scripts/ci/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/ci/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

门禁不会对参数数组隐式执行 shell 解析。命令需要 shell 组合时，在数组中显式调用 shell。

## 理解回执的绑定范围

完成回执记录以下内容：

- 当前 `HEAD` 和选定的基准提交。
- 修改文件集合。
- 已跟踪文件的干净工作树状态。
- 每项声明检查的退出状态和有限长度输出。

后续提交或已跟踪文件的修改会使回执失效。发生这类变化后，重新运行适用检查。

门禁将 Rust `crates/` 和 Apple 适配器或 App 源码视为产品实现。源码修改需要回归证据；产品修改还需要功能、集成或真机证据。

Cargo 和 Swift 清单、锁文件、工具链配置和 App 的 Xcode 工程属于构建输入，不能当作纯文档修改。删除文件和跨组件职责移动的两端都计入修改。Git 路径与已跟踪状态使用 NUL 分隔记录，避免引号、内嵌空白或任意路径字节隐藏修改。

研究目录中的 `.rs` 文件仍属于研究代码。不能仅因使用 Rust 语法就将文件归为产品实现。

## 使用有针对性的检查

| 修改类型 | 选择的证据 |
| --- | --- |
| 仅文档 | 文档一致性和本地链接检查。 |
| 转换核心 | 针对性回归与功能媒体证据。 |
| Motion Photo | 覆盖受影响的解析、写入、时间或发布行为的公开样本检查。 |
| Photos 交互 | 实际执行所述交互的原生框架或真机证据。 |

不得仅为增加检查数量而运行无关且昂贵的矩阵。完整选择规则见[测试政策](../quality/testing.md)。

## 处理公开和私有媒体

版本化的真实 Motion Photo 位于 `fixtures/`。其他 ProXDR、Apple 功能和仅用于真机验证的样本可以留在 Git 之外。

运行器能读取外部样本时，计划可以用绝对路径引用该文件。仍须遵守测试政策中的样本隐私和产物上传限制。

## 保留历史验证记录

本目录中的带日期文件可能描述旧实现。不得修改旧测量值以符合当前代码。证据变化时，编写新的当前文档或新的带日期记录。

当前文档遵循[技术写作规范](../style-guide.md)。

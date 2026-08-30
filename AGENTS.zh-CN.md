# XDRemux Agent 验收契约

[English](AGENTS.md) | 简体中文

只有必需证据在准确的已提交 `HEAD` 上通过后，Agent 才能声称修改已经完成。

本文档定义仓库验收契约。plan 格式和示例见[验证 runbook](docs/validation/README.md)。

## 必需流程

1. 识别每一条受影响产品链路。
2. 确定每条链路需要的验收证据。
3. 完成目标修改，不加入无关编辑。
4. 提交修改。
5. 创建 completion-gate plan。
6. 针对目标 base 运行 gate。
7. 验证生成的 receipt。
8. 只报告证据真正证明的行为。

示例：

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

compiler pass、parser pass 或 smoke test 不能替代要求的 gate。

## 证据要求

源码修改在行为可测试时必须有针对性 regression check。

production conversion-core 或 app-core 修改还必须有真正到达受影响行为的 functional、integration 或 device evidence。

多个入口同时变化时，每一个受影响入口都要验证。

不要把静态源码检查当成 functional evidence。

不要把 strict ISO parser 通过当成 OPPO 相册渲染证据。

不要只用容器结构证明 Apple Photos 交互式编辑行为。

依赖设备的结论需要 device evidence。必需设备或封闭组件不可用时，只能把结论限制为已经测试的离线行为。

## 范围

默认使用有针对性的验证。

release、跨模块修改或验证框架修改需要更广的仓库验证。

不要为了让 plan 看起来更完整而运行无关的昂贵检查。

## Receipt 完整性

completion receipt 会绑定：

- `HEAD`；
- base commit；
- changed path；
- clean tracked worktree；
- 声明的检查及其结果。

之后的 commit 或 tracked edit 会让 receipt 失效。

completion plan 中所有声明检查都是必需项。

## 媒体和 fixture

公开 Motion Photo fixture 版本化保存在 `fixtures/`。

其他大文件、私有样本、只可真机验证样本或 Apple feature 样本可以保留在 Git 之外。

runner 可以访问外部本地样本时，plan 可以引用它。

## 文档

当前技术文档遵循 [docs/style-guide.md](docs/style-guide.md)。

代码修改改变已记录契约时，先更新英文 canonical 文档，再更新中文版本。

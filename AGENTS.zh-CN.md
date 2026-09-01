# XDRemux Agent 操作契约

[English](AGENTS.md) | 简体中文

本文档是仓库 Agent 的低成本入口。默认保持小 working set，只有受影响契约确实需要更多上下文时才扩展。

不要根据目录名、workflow 名称或旧 PR 重新猜整个系统。使用这些来源：

- [系统架构](docs/architecture.md)：稳定 layer、ownership、依赖规则和 source-of-truth 规则。
- [`docs/agent-map.json`](docs/agent-map.json)：机器可读 capability routing 和长生命周期 branch 职责。
- [迁移路线图](docs/roadmap.md)：migration 和 research promotion gate。
- [验证 runbook](docs/validation/README.md)：evidence class、evidence role、completion plan 和 receipt。
- [执行计划](docs/exec-plans/README.md)：跨 session 或 PR 工作的可恢复持久状态。

## 启动

对于较大的任务：

1. 实时推导 Git 状态，不从散文档复制：

   ```bash
   python3 scripts/agent_context.py status
   ```

   如果当前 branch 没有注册在 `docs/agent-map.json`，用 `--base` 显式传入 intended base。
2. 确定受影响 capability 和 layer。已知 identifier 时使用 `python3 scripts/agent_context.py capability <id>`。
3. 阅读 owning module、相邻测试、相关 fixture 和最窄的当前 normative document。
4. 在假设任一侧包含全部最新知识之前，先比较 active branch 与 intended base。
5. 只有任务涉及 migration、长生命周期 branch 或模型研究时才读取 `docs/roadmap.md`。
6. 只有工作必须跨一次 session/PR、跨层，或 promotion evidence 受阻时才创建 execution plan。
7. 跨层或改变契约的任务在实现之前先定义 acceptance criteria。

默认不要扫描整个仓库。只有证据表明另一层也受影响时才扩大 working set。

## Source of truth

使用最窄的权威来源：

- 已发布行为：当前 release contract、公开文档、实现和匹配证据；
- 稳定架构：`docs/architecture.md`；
- routing metadata 和 branch 职责：`docs/agent-map.json`；
- live branch fact：Git 和 branch 本身，而不是散文档里复制的 HEAD、ahead/behind、crate 清单或 workflow 结果；
- behavioral invariant：standard、fixture、conformance test，以及适用时的 device evidence；
- research claim：model card、data provenance、evaluation code 和当前 measured artifact。

带日期 audit、旧 PR 描述、completed plan 和旧实现属于 evidence，不会自动成为 specification。

任何长生命周期 branch 都不得成为某个稳定系统契约唯一存在的位置。

## 架构 invariant

- 编程语言是 implementation lane，不是 architecture layer。
- format primitive 不拥有 media 或 product policy。
- normalized media semantic 不依赖 CLI、App 或 research code。
- planning 保持 deterministic，并且不做 platform I/O。
- 优先使用 operation-scoped adapter capability，而不是巨型 backend。
- publication、provenance、collision handling 和 crash recovery 都是 correctness contract。
- product shell 不重新实现 parser 或 media semantic。
- research output 在 promotion gate 通过前只是 candidate。
- 不要仅为了对应旧 Swift 目录而创建 Rust crate。
- 不要把仓库级规则复制到 path-specific instruction。只有确实存在无法由 owner contract 或 test 表达的局部 invariant 时才增加局部指令。

一个修改无法放入这些边界时，先解决 ownership，而不是再增加一个 special case。

## 完成契约

只有必需证据在准确的已提交 `HEAD` 上通过后，Agent 才能声称修改已经完成。

必需流程：

1. 识别每一项受影响 capability 和 product path。
2. 根据结论选择必需的 regression、conformance、functional、integration 或 device evidence。
3. 把 required/promotion evidence 与 diagnostic probe 分开。诊断探针即使有用或变绿，也不会因此自动成为 acceptance evidence。
4. 完成目标修改，不加入无关编辑；契约变化时同步更新文档。
5. 提交修改。
6. 针对 intended base 使用包含全部 required check 的 plan 运行 `scripts/agent_completion_gate.py`。
7. 验证准确 `HEAD` 对应的 receipt。
8. 只报告证据真正证明的行为，并明确 residual gap。

示例：

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

当修改结论本身是 functional 时，compiler、parser、smoke、static check 或 diagnostic workflow 都不能替代 functional evidence。

structural、ImageIO、renderer、integration 和 device evidence 必须保持区分。Apple Photos 或设备行为需要真正到达该 consumer 的证据。必需环境不可用时，应限制结论范围，而不是标记为完成。

completion plan 中声明的所有检查都是必需项。之后的 commit 或 tracked edit 会让 receipt 失效。

## Migration 与研究

每一个从 Swift/Python 迁移到 Rust 的 capability，都要在 roadmap、PR、当前契约或 active execution plan 中记录四项事实：normalized contract、oracle/evidence、Rust owner 和 promotion evidence。

cross-implementation parity 很有价值，但旧实现与 external standard 或更强证据冲突时，它本身不够。

v1.4 Swift/Python 线是有边界的 released reference。不要只为了与 Rust 保持对称，就继续在旧实现上增加大型新架构。

model、learned heuristic 或 research-only producer 在 `docs/roadmap.md` 中适用的 gate 通过之前，必须留在 production default 之外。稳定 research protocol 应归入研究/model 文档，而不是通用 development guide。

retire 一个长生命周期 branch 之前，把 branch-only 的稳定知识 promotion 到 code、test、model card 或 normative documentation。有继续价值的带日期 experiment 作为 historical record 保存。

## 媒体与文档

公开 Motion Photo fixture 版本化保存在 `fixtures/`。大型私有、device-only 或 Apple-feature 样本可以留在 Git 之外，但对应 validation path 必须记录其 provenance 和用途。

当前技术文档遵循 [docs/style-guide.md](docs/style-guide.md)。先修改英文 canonical document，再同步中文翻译。

不要把临时 chain-of-thought 或 session scratch 持久化到仓库。只保存其他维护者或 Agent 以后必须恢复的 decision、contract、provenance、reusable evidence 和 resumable plan。

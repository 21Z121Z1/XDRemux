# XDRemux 执行计划

[English](README.en.md) | 简体中文

只有当工作需要跨越一次 Agent session 或一个 PR 时才创建执行计划。一个小改动如果可以从 PR 和测试中直接理解，就不要额外建计划。

执行计划保存的是可恢复的工作状态。它不是对话记录、思维链、日记，也不能替代规范架构。

## 什么时候创建

至少满足一个条件时创建 active plan：

- 工作跨多个 capability 或架构层；
- 预计需要多个 commit 或 PR；
- 必需证据受 runner、真机、私有 fixture 或外部 consumer 阻塞；
- 另一个 Agent 必须能够在不重建聊天历史的情况下继续；
- 工作包含需要逐步达到的 migration 或 research promotion gate。

一个边界清楚的单 PR 任务使用 PR task ledger 即可。

## 位置与生命周期

第一次真正需要 active plan 时，把它放在 `docs/exec-plans/active/`。只有在完成证据已经记录后，才把完成的计划移到 `docs/exec-plans/completed/`。

不要因为以后可能还有相关工作而一直把计划保持为 active。当前 objective 完成后关闭该计划；目标实质变化时新建计划。

## 必需字段

每份计划必须包含：

- **Status**：`proposed`、`active`、`blocked`、`complete` 或 `superseded`。
- **Target capability / layer**：`docs/agent-map.json` 中的 identifier 和所属架构层。
- **Branch / intended base / last verified HEAD**：使用准确 ref，不用模糊日期代替。
- **Objective**：产品或架构结果，而不是待修改文件列表。
- **Invariant**：必须保持成立的行为或边界。
- **Known facts and evidence**：可复现事实，并链接到代码、fixture、model card、测试或 validation record。
- **Decisions**：可长期复用的结论和证据。记录结论，不保存私有推理草稿。
- **Work sequence**：带依赖关系和 acceptance check 的有序步骤。
- **Completed evidence**：已经得到的准确命令、workflow check、receipt 或真机证据。
- **Residual gaps**：尚未证明什么，以及什么证据可以关闭缺口。
- **Next action**：一个具体、可恢复的下一动作。

## 更新纪律

当 decision、已验证事实、blocker、promotion state 或 next action 变化时更新计划。不要为了适应后来的结论而改写已经验证过的历史。

可以低成本实时推导的易变事实不要手工复制。分支 divergence 和当前 HEAD 使用：

```bash
python3 scripts/agent_context.py status
```

capability ownership 和 evidence routing 使用：

```bash
python3 scripts/agent_context.py capability engine.plan
```

计划可以记录 last verified HEAD 以便复现，但 branch 移动后不得把旧值写成“当前状态”。

## 最小模板

```markdown
# <Outcome>

Status: active
Target capability / layer: engine.plan / Layer 3
Branch: <branch>
Intended base: <base>
Last verified HEAD: <sha>

## Objective

## Invariant

## Known facts and evidence

## Decisions

## Work sequence

1. <step> — acceptance: <check>

## Completed evidence

## Residual gaps

## Next action
```

如果计划发现了稳定的仓库级规则，应把规则提升到 architecture、validation contract、model card 或其他规范 owner 中。不要让稳定系统知识只留在 completed plan 里。

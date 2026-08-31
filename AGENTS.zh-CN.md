# XDRemux Agent 操作与验收契约

[English](AGENTS.md) | 简体中文

Agent 在修改代码之前必须先理解受影响的系统边界，并且只有必需证据在准确的已提交 `HEAD` 上通过后，才能声称修改已经完成。

使用[系统架构](docs/architecture.md)确定 ownership 和 dependency。Rust 迁移与 Photographic Styles 研究 promotion 见[迁移路线图](docs/roadmap.md)。completion plan 格式和证据示例见[验证 runbook](docs/validation/README.md)。

## 启动流程

对于较大的任务，在大范围搜索或实现之前先建立仓库状态：

1. 记录当前 branch、准确 `HEAD`、intended base 和 clean/dirty state。
2. 阅读 `docs/architecture.md`。
3. 任务涉及 Rust rewrite、长生命周期 branch 或模型研究时，阅读 `docs/roadmap.md`。
4. 确定受影响 capability identifier 和架构层。
5. 在假设任一侧包含全部当前知识之前，比较 active branch 和 intended base。
6. 阅读 owning module、相邻测试、相关 fixture 和当前 normative document。
7. 跨层或改变契约的任务，在实现之前先定义 acceptance criteria。

除非任务本身就是全仓库审计，否则不要一开始扫描整个仓库。只有证据表明另一层也受影响时才扩展 working set。

## Source-of-truth 纪律

根据问题使用真正权威的来源：

- 已发布用户行为：当前 release contract、当前公开文档、实现和匹配证据；
- 架构 ownership：`docs/architecture.md` 和当前代码边界；
- active branch fact：该 branch 加上与 intended base 的显式比较；
- behavioral invariant：standard、fixture、conformance test，以及适用时的 device evidence；
- research claim：model card、data provenance、evaluation code 和当前 measured artifact；
- 带日期的 audit 和旧 PR 描述：除非当前文档采用其结论，否则属于 historical evidence。

旧实现是有价值的 migration oracle，但不会自动成为 specification。

任何长生命周期 branch 都不得成为某个稳定系统契约唯一存在的位置。

## 必需实现流程

1. 识别每一项受影响 capability 和 product path。
2. 确定每条路径的 acceptance criteria 和 required evidence。
3. 完成目标修改，不加入无关编辑。
4. 增加或更新 targeted regression / conformance evidence。
5. 契约变化时更新当前 normative documentation。
6. 提交修改。
7. 创建 completion-gate plan。
8. 针对 intended base 运行 gate。
9. 验证生成的 receipt。
10. 只报告证据真正证明的行为。

示例：

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

compiler pass、parser pass 或 smoke test 不能替代要求的 gate。

## 架构规则

实现过程中遵守 `docs/architecture.md` 中的 dependency rule。

尤其是：

- 不要按编程语言组织架构；
- 不要把 product policy 放进 parser、codec adapter、native helper、CLI code 或 model predictor；
- planning 保持 deterministic，并且不做 platform I/O；
- 优先使用 operation-scoped adapter capability，而不是巨型 backend；
- 不要仅仅为了对应旧 Swift 目录而创建 Rust crate；
- publication、provenance、collision handling 和 crash recovery 都属于 correctness contract；
- research model output 在 promotion gate 通过前只是 candidate；
- 不要只为了方便迁移而增加永久 cross-language runtime dependency。

一个修改无法自然放入当前架构时，先判断应该由哪一层拥有该行为。不要用新的 special case 掩盖 ownership ambiguity。

## 证据要求

每一项源码修改都必须有针对性的 regression 或 conformance check，并且该检查必须能在原始缺陷或原始契约违规存在时失败。

每一项 production conversion-core 或 app-core 修改还必须有真正到达受影响行为的 functional、integration 或 device evidence。

多个入口同时变化时，每一个受影响入口都必须验证。

不要把静态源码检查当成 functional evidence。

不要为了满足 gate，把 static check 改名成 regression 或 functional check。

strict ISO parser 通过本身不能作为 OPPO 相册行为的验收证据。structural、ImageIO、renderer 和 device evidence 必须保持区分。

不要只用 container structure 证明 Apple Photos 交互式编辑行为。

依赖设备的产品结论需要 device evidence。必需设备或封闭组件不可用时，必须把依赖设备的结论标记为 blocked，或者明确把结论限制为已经测试的 offline behavior。没有 device evidence 时，不得声称依赖设备的结论已经完成验证。

completion plan 中声明的所有检查都是必需项。

## 迁移证据

一个 capability 从 Swift/Python 迁移到 Rust 时，记录：

1. normalized contract；
2. 用作 oracle 的旧实现或 external evidence；
3. Rust owner；
4. promotion evidence。

旧实现与 external standard 或更强证据冲突时，cross-implementation parity 本身不够。

v1.4 Swift/Python 线是有边界的 released reference。不要仅为了与 Rust 保持 implementation symmetry，就继续在旧实现上增加大型新架构。

## 研究 Promotion

model、learned heuristic 或 research-only producer 在 `docs/roadmap.md` 中适用的 promotion gate 通过之前，必须留在 production default 之外。

training provenance、leakage control、held-out/OOD result、consumer correlation、uncertainty/fallback behavior、end-to-end evidence 和 operational budget 必须保持区分。

不要把更低的 offline loss 当成足够的 product evidence。

## 范围

默认使用有针对性的验证。

release/preflight、跨模块修改、architecture/control-plane 修改或 verification framework 修改需要更广的仓库验证。

不要为了让 plan 看起来更完整而运行无关的昂贵检查。

## Receipt 完整性

completion receipt 会绑定：

- `HEAD`；
- base commit；
- changed path；
- clean tracked worktree；
- 声明的检查及其结果。

之后的 commit 或 tracked edit 会让 receipt 失效。

## Branch 生命周期

删除或 retire 一个长生命周期 branch 之前：

1. 与 intended destination 比较；
2. 找出只有该 branch 才存在的 implementation、contract、evidence 或 provenance；
3. 把可复用的当前知识 promotion 到 code、test、model card 或 normative documentation；
4. 有继续价值的带日期 experiment 作为 historical record 保存；
5. 确认没有稳定 contract 依赖 branch-only knowledge 后再 retire。

实现已经 merge 还不够。如果维护所需的 reasoning 或 evidence boundary 只存在于旧 PR、branch 或聊天记录中，知识迁移仍未完成。

## 媒体和 Fixture

公开 Motion Photo fixture 版本化保存在 `fixtures/`。

其他大文件、私有样本、只可真机验证样本或 Apple feature 样本可以保留在 Git 之外。

runner 可以访问外部本地样本时，verification plan 可以引用它。

`.codex/verification-receipts/` 下的 verification receipt 继续由 Git 忽略。

## 文档

当前技术文档遵循 [docs/style-guide.md](docs/style-guide.md)。

代码修改改变已记录契约时，先更新英文 canonical 文档，再更新中文版本。

不要把临时 chain-of-thought 或 session scratch 持久化到仓库。只保存其他维护者或 Agent 以后必须恢复的 decision、contract、provenance、reusable evidence 和 plan。

# XDRemux 迁移路线图

[English](roadmap.en.md) | 简体中文

本文档定义从已经发布的 v1.4 Swift/Python 线迁移到 Rust 产品线的路径，也定义 Photographic Styles 研究进入产品的 promotion 路径。

稳定 ownership 和依赖规则见[系统架构](architecture.md)。机器可读 branch/capability routing 见 [`agent-map.json`](agent-map.json)。本文档记录阶段和 promotion gate，不保存高频变化的 Git 状态。

## 目标

目标不是把每一个 Swift 或 Python 函数翻译成 Rust。

目标是在保留或改进已经验证的 XDRemux 产品契约的同时，把稳定媒体语义和产品编排迁移到更小、更显式的 Rust 架构中。

迁移必须减少四类歧义：

- 某个行为由哪一层拥有；
- 某个结论到底以哪个实现或证据来源为准；
- 某个 capability 在 promotion 前需要哪些证据；
- 哪些当前事实必须实时推导，而不是复制进散文档。

## 动态状态规则

不要在本路线图中维护当前 `HEAD`、ahead/behind、完整 workspace membership、changed-file list 或当前 workflow result。

它们从 Git、code、manifest 和 CI 实时推导：

```bash
python3 scripts/agent_context.py status
python3 scripts/agent_context.py capability adapter.codec
```

在 Rust 分支上，该分支的 `Cargo.toml` 是当前 workspace membership 的权威来源。crate 已经存在只表示实现边界存在，不表示该 capability 已经通过 promotion evidence。

在当前架构里程碑上，纯语义基础已经到达 `xdremux-engine`，`xdremux-codec` 是第一个具体的第 4 层 adapter boundary。其 provider 仍必须通过 operation contract 和真实 runtime evidence 才能 promotion。

## Branch 生命周期

稳定的长生命周期 branch role 存在 `docs/agent-map.json`：

- `main`：已经发布的 v1.4 reference 和 shared control-plane destination；
- `feat/rust-xdremux-format`：主动 Rust migration implementation line；
- `codex/reverse-key1-oppo-solver`：Photographic Styles research line。

Branch name 只是引用，不是架构。当前 Rust branch 名称已经比实际承担范围窄得多。

每个长生命周期 branch 都必须明确四件事：role、intended base、promotion gate 和 retirement condition。不要仅为了代表一个 architecture layer 再创建长生命周期分支。

## 迁移 invariant

每一个迁移到 Rust 的 capability 都必须在 PR、当前契约或 active execution plan 中记录四项内容：

1. normalized contract；
2. 用作 oracle 的旧实现或外部证据；
3. Rust owner；
4. promotion evidence。

只要缺一项，即使 Rust 已经编译通过或 diagnostic workflow 通过，迁移仍不完整。

## Phase 1：冻结 v1.4 行为契约

目的：把已发布 Swift/Python 线变成有边界的 reference，而不是无限继续演化的竞争实现。

必需工作：

- 保持 v1.4 公开文档准确；
- 保留真实 Motion Photo fixture corpus 和 hash；
- 保留 conversion safety 和 publication regression；
- 保留 Rust conformance 所需行为；
- 必要时修复已发布安全缺陷；
- 避免在旧实现线继续无关大型功能开发。

Exit criteria：

- 每个 Rust migration area 都能指向稳定 test、fixture、standard 或当前 product contract；
- 没有主动迁移只依赖长分支里的未记录行为。

## Phase 2：完成纯 Rust 语义核心

目的：在扩展 product shell 之前，把第 1～3 层做成显式且可独立测试的结构。

必需属性：

- parser 对 malformed bounds 和 length fail closed；
- vendor metadata 转成 normalized semantic model；
- Gain Map semantic 与 container-writing side effect 分离；
- Motion Photo parsing 产出稳定 topology/timing/payload model；
- classification 消费 normalized asset fact；
- `xdremux-engine` 根据 source fact、request 和 capability inventory 产生 deterministic plan；
- planning 保持无 platform I/O。

Exit criteria：

- 每个已迁移 semantic capability 都有针对性 Rust test；
- v1.4 适合作 oracle 的地方通过 cross-implementation vector；
- 旧实现不足以成为规范的地方有 external-standard test；
- 必需 Rust semantic path 在 runtime 不依赖 Swift 或 Python。

## Phase 3：通过 Operation Adapter 让 Plan 可执行

目的：让 engine plan 真正可执行，同时不重建 platform monolith。

Engine 已经为 raster decoding、Gain Map tile encoding、RAW processing、consumer validation、Photographic Styles 和 Portrait 定义 operation-scoped port。`xdremux-codec` 是第一个具体 provider boundary，并体现目标依赖方向：provider → engine port，而不是 engine → concrete provider。

必需工作：

- 用真实 runtime behavior 验证 concrete codec/provider capability，而不只相信 library advertisement；
- 提供 standard HDR conversion 所需 operation；
- 在 product composition root 组合 provider，不把 adapter instance 塞进 planning fact；
- capability discovery 只表达事实，与 policy 分离；
- 每个 adapter 通过自己的 operation contract 测试；
- native 或 closed-framework code 留在纯 semantic core 之外。

Provider probe 可以为了刻画 dependency behavior 临时 patch CI checkout，但这种 probe 只属于 diagnostic。只有发现被固化进真实 implementation/test contract 后，才能成为验收证据。

Exit criteria：

- standard HDR plan 可以通过显式 adapter 完整执行；
- 必需 provider capability 在支持的 runtime environment 中通过可复现 operation-contract test；
- 缺少 capability 时通过明确 planner/composition error 失败；
- adapter-specific failure 不会静默改变 engine policy；
- canonical provider test 不依赖隐藏的 CI-only source patch。

## Phase 4：恢复完整 Asset 与 Publication 语义

目的：从正确的 still-image path 扩展到完整 XDRemux asset model。

必须恢复 v1.4 中很容易在 rewrite 中丢失的行为：

- Motion Photo cover-frame timing；
- 需要保留的 compressed video/audio passthrough；
- Apple Live Photo shared asset identity；
- deterministic output naming；
- pair provenance；
- collision handling；
- crash-recoverable pair publication；
- batch resume rule；
- source preservation 和 destructive-operation safety。

这些都是 product correctness contract，不是 CLI convenience。

Exit criteria：

- public Motion Photo fixture 通过 Rust path；
- Live Photo pair identity 和 timing 符合契约；
- pair-publication regression 覆盖 crash/collision/provenance；
- 需要该结论时，PhotoKit 或等价 integration evidence 验证生成 pair。

## Phase 5：加入 Apple 专用产品 Adapter

目的：把 Apple-only behavior 保留在显式 outgoing capability boundary。

Photographic Styles 和 Portrait 不应作为 platform assumption 进入纯 engine。

必需工作：

- 为 engine port 定义具体 Apple adapter composition；
- 如仍需 private framework，保留 runtime ABI negotiation 和 fail-closed behavior；
- consumer validation 与 container construction 继续作为不同 operation；
- device-dependent claim 继续要求 device evidence。

Exit criteria：

- Apple feature request 是显式 engine requirement；
- 非 Apple composition 可以不带 Apple framework 构建和使用 standard core；
- 每个 promotion 的 Apple feature 都通过 consumer 和 device evidence。

## Phase 6：建立 Rust 产品 Shell

目的：让 Rust 成为产品线，而不是一组 conformance crate。

必需工作：

- 暴露稳定 library composition root；
- engine request model 稳定后再加入支持的 CLI surface；
- 把 structured engine event 映射成 user output，但不把 terminal policy 放进 core；
- 保留 output-safety semantic；
- 如果 macOS app 继续使用 SwiftUI，通过窄 library/FFI boundary 集成；
- 记录可复现 packaging 和 release artifact。

不要从复制旧 CLI 全部参数开始。只有对应受支持 engine contract 的 option 才 promotion。research-only 或 obsolete control 应直接 retire，而不是自动带到新产品线。

Exit criteria：

- standard conversion、Motion Photo/Live Photo、classification 和选定 Apple feature 都能从目标产品入口调用；
- CLI 和 app 不重新实现 media policy；
- release packaging 可复现，并由 CI 绑定准确 `HEAD`。

## Phase 7：Rust Release Promotion

只有 exact release candidate 满足每个适用 evidence gate，Rust release 才能取代 v1.4。

最少 release evidence：

- format/parser regression suite；
- Rust semantic unit/integration test；
- 有价值的 cross-implementation conformance vector；
- public Motion Photo real-fixture gate；
- standard HDR real-fixture conversion 和 container validation；
- output-safety 和 publication regression；
- CLI/product integration test；
- release 中每个 Apple feature 对应的 Apple consumer/device evidence；
- exact-HEAD completion receipt；
- release candidate 上要求的 GitHub Actions 和 CodeQL check。

任何 feature 如果不能满足自己的 evidence gate，就必须排除、标 experimental 或明确限制范围。不能因为 release 其他部分都是绿色，就静默接受它。

## Verification 控制面收敛

迁移期 focused workflow 很有用，因为各 Rust capability 可以独立演化。但它们不能成为 Agent 理解 verification 的唯一方式。

Rust 成为产品线的过程中，逐步收敛到这些属性：

- 普通 preflight 有一个明确记录的 repository-level Rust verification entry point；
- focused capability check 仍能独立调用；
- workflow/check 名称能表达它是 required、promotion evidence 还是 diagnostic；
- diagnostic probe 在转成稳定 contract test 前不成为 required merge check；
- exact release/product gate 组合适用 capability check，而不是复制其逻辑。

目标不是机械减少 workflow 文件数量，而是让“什么证据能证明这个修改”只有一个清晰答案。

## Photographic Styles 研究 Promotion

研究线使用独立 promotion ladder。模型精度工作可以和 Rust rewrite 并行，但产品 promotion 独立验收。

稳定 model contract 归入 model card。稳定 dataset/training/evaluation procedure 归入研究线专门 research documentation。不要让 research protocol 只存在于通用 development guide 或聊天记录。

一个 model 或 learned component 必须按顺序通过：

1. **Data provenance**：每个训练/评估输入都有已知 source、license/private-data status、identity hash 和 label/teacher provenance。
2. **Leakage control**：calibration、training、held-out 和最终 locked set 不共享会破坏指标有效性的 source session 或 derived copy。
3. **Primary-only robustness**：optional modality 存在时能提高结果，但缺失时不会让普通图片路径崩溃。
4. **Held-out and OOD**：candidate 在预定义 metric 和重要 device/content strata 上优于 accepted baseline。
5. **Consumer correlation**：更低 parameter loss 同时改善产品真正关心的 renderer/consumer response。
6. **Bounded uncertainty**：产品存在可测量 reject/fallback rule，处理超出 supported envelope 的情况。
7. **End-to-end product evidence**：生成 asset 通过 container、native consumer 和 device test。
8. **Operational budget**：runtime、memory、model size 和 failure behavior 满足产品目标。

只有全部 gate 通过后，learned component 才能从 `research.styles-model` 迁移成 production adapter capability。

Engine 通过稳定 adapter contract 消费已经 promotion 的 capability，不 import training code，也不需要知道 model 如何训练。

## Branch Retirement 与知识 Promotion

删除或替换长生命周期 branch 之前：

1. 与 intended destination 比较；
2. 找出其他地方不存在的 commit、contract、protocol 或 evidence；
3. 把稳定知识 promotion 到 code、test、model card、research doc 或 normative document；
4. 有继续价值的带日期 experiment evidence 作为 historical record 保存；
5. 确认没有稳定 contract 依赖 branch-only knowledge；
6. 然后才 retire branch。

实现已经 merge 还不够。如果维护该实现需要的 reasoning boundary、acceptance contract 或 provenance 只存在于旧 PR、混合 development guide 或聊天记录里，知识迁移仍未完成。

## Agent 执行模式

一个边界清楚的 PR 使用 `.github/pull_request_template.md` 作为紧凑 task ledger。

跨 session/PR 的工作使用[执行计划契约](exec-plans/README.md)。不要为了保存临时 chain-of-thought 或 session scratch 而创建仓库文件。

真正需要持久保存的状态是：target capability、准确 ref、invariant、evidence、decision、有序工作、promotion state、residual gap，以及一个可恢复 next action。

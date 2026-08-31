# XDRemux 迁移路线图

[English](roadmap.en.md) | 简体中文

本文档定义从已经发布的 v1.4 Swift/Python 线迁移到 Rust 产品线的当前计划，也定义 Photographic Styles 研究线进入产品的条件。

Ownership 和依赖规则见[系统架构](architecture.md)。本文档记录迁移状态和 promotion gate。

## 目标

目标不是把每一个 Swift 或 Python 函数翻译成 Rust。

目标是在保留或改进已经验证的 XDRemux 产品契约的同时，把稳定媒体语义和产品编排迁移到更小、更显式的 Rust 架构中。

迁移必须减少三类歧义：

- 某个行为到底由哪一层拥有；
- 某个结论到底以哪个实现为准；
- 某个 capability 在 promotion 之前到底需要哪些证据。

## 当前分支职责

| Branch | 职责 | Canonical for | Not canonical for | Retirement condition |
| --- | --- | --- | --- | --- |
| `main` | 已发布 v1.4 维护/参考线 | v1.4 公开行为和 release artifact | 未来 Rust 架构 | Rust release 已覆盖相关产品契约，而且不再需要 v1.4 维护 |
| `feat/rust-xdremux-format` | 主动 Rust rewrite | 当前 Rust 实现工作 | Rust release 出现之前的已发布用户行为 | rewrite 达到产品就绪后重命名或合并进 Rust release line |
| `codex/reverse-key1-oppo-solver` | Photographic Styles 研究线 | 该分支上的当前研究实现和实验 | production default 和已发布质量结论 | 有价值研究经过显式 gate 被 promotion，或作为历史证据归档 |

Branch name 只是引用，不是架构。当前 Rust branch name 已经比它实际承担的范围窄得多。

开始处理任何长生命周期分支之前，都先与 intended base 比较。不要把 ahead/behind 数量写进规范文档，因为每次 commit 后它都会失效。

## 当前 Rust 基础

Rust workspace 当前包含：

- `xdremux-format`；
- `xdremux-metadata`；
- `xdremux-hdr`；
- `xdremux-container`；
- `xdremux-heif`；
- `xdremux-motion-photo`；
- `xdremux-classification`；
- `xdremux-engine`。

该分支还包含 Swift/Python → Rust conformance oracle 和定向 Rust CI workflow。

它已经不再是 format parser 实验。下一步应该围绕这套基础补齐完整架构，而不是继续增加直接函数移植。

## 迁移 invariant

每一个迁移到 Rust 的 capability 都必须记录四项内容：

1. normalized contract；
2. 用作 oracle 的旧实现或外部证据；
3. Rust owner；
4. promotion evidence。

只要缺一项，即使 Rust 已经编译通过，迁移也不完整。

## Phase 1：冻结 v1.4 行为契约

目的：把已发布 Swift/Python 线变成有边界的 reference，而不是一个无限继续演化的竞争实现。

必需工作：

- 保持 v1.4 公开文档准确；
- 保留真实 Motion Photo fixture corpus 和 hash；
- 保留 conversion safety 和 publication regression；
- 保留 Rust conformance 所需的准确行为；
- 必要时修复已发布安全缺陷；
- 避免在旧实现线继续进行无关的大型功能开发。

Exit criteria：

- 每个 Rust migration area 都能指向稳定 test、fixture、standard 或当前 product contract；
- 没有任何主动迁移只依赖某个长分支里的未记录行为。

## Phase 2：完成纯 Rust 语义核心

目的：先完成架构第 1～3 层，再扩展产品 shell。

现有 crate 已经覆盖大部分预期语义分区。新的工作重点应该是缺失契约和组合，而不是 crate 数量。

必需属性：

- parser 对 malformed bounds 和 length fail closed；
- vendor metadata 转成 normalized semantic model；
- Gain Map semantic 与 container-writing side effect 分离；
- Motion Photo parsing 产出稳定 topology/timing/payload model；
- classification 消费 normalized asset fact；
- `xdremux-engine` 根据 source fact、request 和 capability inventory 产生 deterministic plan；
- planning 保持无 platform I/O。

Exit criteria：

- 每个已经迁移的 semantic capability 都有针对性 Rust test；
- v1.4 适合作 oracle 的地方都通过 cross-implementation vector；
- 旧实现不足以成为规范的地方有 external-standard test；
- 必需 Rust semantic path 在 runtime 不依赖 Swift 或 Python。

## Phase 3：实现执行 Adapter，但不重建巨型单体

目的：让 plan 可执行，同时保持 platform/library operation 可替换。

当前 engine 已为 raster decoding、Gain Map tile encoding、RAW processing、consumer validation、Photographic Styles 和 Portrait 定义 operation-scoped port。继续沿用这个模型。

必需工作：

- 为 standard HDR conversion 所需操作提供具体 codec 和 platform adapter；
- 只有稳定边界已经明确时才固定 request/output type；
- capability discovery 只表达事实，与 policy 分离；
- 每个 adapter 通过自己的 operation contract 测试；
- native 或 closed-framework code 留在纯 semantic core 之外。

不要引入万能 platform backend。不要因为一个 adapter 存在，就让无关 operation 强制依赖它。

Exit criteria：

- standard HDR plan 可以通过显式 adapter 完整执行；
- 缺少 capability 时通过明确 planner/composition error 失败；
- adapter-specific failure 不会静默改变 engine policy。

## Phase 4：恢复完整 Asset 与 Publication 语义

目的：从正确的 still-image core 扩展到完整 XDRemux asset model。

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

- public Motion Photo fixture 全部通过 Rust path；
- Live Photo pair identity 和 timing 符合契约；
- pair publication regression 覆盖 crash/collision/provenance；
- 需要该结论时，macOS PhotoKit 或等价 integration evidence 验证生成 pair。

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
- 每一个 promotion 的 Apple feature 都通过 consumer 和 device evidence。

## Phase 6：建立 Rust 产品 Shell

目的：让 Rust 成为产品线，而不是一组 conformance crate。

必需工作：

- 暴露稳定 library composition root；
- engine request model 稳定后再加入支持的 CLI surface；
- 把 structured engine event 映射成 user output，但不把 terminal policy 放进 core；
- 保留当前 output-safety semantic；
- 如果 macOS app 继续使用 SwiftUI，通过窄 library/FFI boundary 集成；
- 记录 packaging 和 release artifact。

不要从复制旧 CLI 全部参数开始。只有对应受支持 engine contract 的 option 才 promotion。research-only 或 obsolete control 应直接 retire，而不是自动带到新产品线。

Exit criteria：

- standard conversion、Motion Photo/Live Photo、classification 和选定 Apple feature 都能从目标产品入口调用；
- CLI 和 app 不重新实现 media policy；
- release packaging 可复现，并由 CI 绑定准确 `HEAD`。

## Phase 7：Rust Release Promotion

只有 exact release candidate 满足所有适用 evidence class，Rust release 才能取代 v1.4。

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

## Photographic Styles 研究 Promotion

研究分支使用单独的 promotion ladder。模型精度工作可以和 Rust rewrite 并行，但产品 promotion 独立验收。

### 当前研究状态

当前研究线包括：

- primary-image model path；
- 可选 RAW-derived linear RGB input；
- 可选 Gain Map input；
- 显式 modality mask 和 modality dropout；
- public synthetic/content-domain pretraining；
- private native/teacher-labelled training support；
- Core ML export 和 Swift research integration；
- held-out、OOD、cascade 和 consumer-oriented evaluation tool。

这些是 research infrastructure，不等于 production validation。

### Promotion gate

一个 model 或 learned component 必须按顺序通过：

1. **Data provenance**：每一个训练/评估输入都有已知 source、license 或 private-data status、identity hash 和 label/teacher provenance。
2. **Leakage control**：calibration、training、held-out 和最终 locked set 不共享会破坏指标有效性的 source session 或 derived copy。
3. **Primary-only robustness**：optional modality 存在时能提高结果，但缺失时不会让普通图片的必需路径崩溃。
4. **Held-out and OOD**：candidate 在预先定义 metric 和重要 device/content strata 上优于当前 accepted baseline。
5. **Consumer correlation**：更低 parameter loss 同时改善产品真正关心的 renderer/consumer response。
6. **Bounded uncertainty**：产品存在可测量的 reject/fallback rule，处理超出 model supported envelope 的情况。
7. **End-to-end product evidence**：实际生成 asset 通过 container、native consumer 和 device test。
8. **Operational budget**：runtime、memory、model size 和 failure behavior 满足产品目标。

只有全部 gate 通过后，model 才能从 `research.styles-model` 迁移成 adapter 或 engine 可见的 production capability。

Engine 应通过稳定 adapter contract 消费已经 promotion 的 capability。Engine 不应 import training code，也不需要知道 model 是如何训练的。

## Branch 生命周期规则

每一个长生命周期 branch 都必须在当前文档或 PR 描述中明确四个事实：

- role；
- intended base；
- promotion gate；
- retirement condition。

删除 branch 之前：

1. 与 intended destination 比较；
2. 找出其他地方不存在的 commit 或 contract；
3. 把有价值的当前知识 promotion 到 code、test、model card 或 normative document；
4. 有继续价值的带日期 experiment evidence 作为 historical record 保存；
5. 确认没有稳定 contract 依赖 branch-only knowledge 后再删除。

实现已经 merge 还不够。如果维护该实现需要的 reasoning、acceptance boundary 或 provenance 只存在于旧 PR 或聊天记录里，知识迁移仍未完成。

## Agent 执行模式

较大的 migration work 在 PR 描述或 working note 中使用这个紧凑 task ledger：

- **Target capability**：`architecture.md` 中一个或多个 identifier。
- **Base and branch**：准确 ref 和 merge base。
- **Current owner**：当前拥有该行为的 source file/crate。
- **Invariant**：必须继续成立的行为。
- **Oracle/evidence**：standard、fixture、v1.4 behavior、device result 或 measured research baseline。
- **Change boundary**：允许修改的 layer。
- **Acceptance checks**：promotion 需要的 command/workflow。
- **Residual gaps**：当前环境无法证明的事实。

不要为了保存临时 chain-of-thought 或 session note 而创建仓库文件。只有其他维护者或 Agent 之后必须恢复的 decision、contract、provenance、reusable evidence 和 plan 才应该持久化。

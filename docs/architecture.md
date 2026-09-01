# XDRemux 系统架构与 Agent 地图

[English](architecture.en.md) | 简体中文

本文档定义维护者和 Agent 用来把 XDRemux 理解为一个完整系统的稳定架构模型。

ownership 和依赖规则以本文档为准。机器可读 routing 见 [`agent-map.json`](agent-map.json)。promotion 和 retirement 规则见[迁移路线图](roadmap.md)。操作与验收契约见 [AGENTS.md](../AGENTS.zh-CN.md)。

## 系统模型

XDRemux 不是一个 Swift 项目、Python 项目、Rust 项目或 Photographic Styles 研究项目。它们都是同一套媒体转换系统中的实现线或研究线。

稳定架构由媒体语义、能力边界、产品契约和证据定义。编程语言、branch、目录、crate 或 workflow 名称本身都不是架构层。

整个系统有两个彼此正交的平面：

- **产品平面**：把输入 asset 和请求转换成已经验证的输出 asset；
- **控制平面**：记录契约、routing metadata、fixture、一致性证据、研究 provenance、promotion state 和验收状态。

研究可以向产品平面提出新行为候选，但研究不拥有产品策略。

## 稳定知识与实时状态

稳定知识必须和每次 commit 都会变化的事实分开。

稳定知识放在 normative document、test、model card 或 `docs/agent-map.json` 中，例如 capability identifier、layer ownership、branch role、invariant、evidence requirement 和 promotion rule。

实时状态必须从仓库推导，例如当前 `HEAD`、ahead/behind、workspace membership、changed path、workflow result 和 diagnostic probe 的最新结果。

Git、manifest、code 或 CI 能直接回答的问题，不要再把实时状态复制进 architecture 文档。使用：

```bash
python3 scripts/agent_context.py status
python3 scripts/agent_context.py capability engine.plan
```

这样实现分支移动时，正确的架构文档不会因为手抄状态而立即过期。

## 抽象塔

从下往上阅读产品平面。高层可以依赖低层。低层不得依赖产品 shell 或研究策略。

### 第 0 层：证据与外部契约

这一层回答：**什么必须一直成立？**

它包括：

- ISO/TS 21496-1 及其他外部格式契约；
- 公开 fixture 和 fixture hash；
- 公开 fixture 不可行时使用的私有或真机证据；
- 跨实现 conformance vector；
- 针对已知缺陷的 regression test；
- consumer 和 device validation receipt。

证据不决定实现结构，但会约束所有实现。

### 第 1 层：二进制与格式原语

这一层回答：**文件中有哪些字节，它们怎样表示？**

职责包括：

- endian-safe byte access；
- FourCC 和 ISO-BMFF box model；
- hardened parser 和 constructor；
- Exif/TIFF 解析和 orientation；
- JPEG 和 HEVC 结构解析；
- bounds 和 overflow validation。

当前 Rust 实现以 `xdremux-format` 和 `xdremux-heif` 为中心。Swift v1.4 reference 中的对应职责主要位于 `Sources/XDRemuxCore/HEIF/`、metadata helper 以及相邻格式代码中。

这一层不得知道 CLI 参数、Apple Photos 行为、模型选择或 batch publication policy。

### 第 2 层：归一化媒体语义

这一层回答：**这个 asset 表达什么含义？**

职责包括：

- OPPO/OnePlus/realme 私有 HDR metadata 的解释；
- Gain Map 参数和 EDR 语义；
- vendor resource extraction；
- Motion Photo topology、timing、payload 和 vendor metadata；
- photo asset classification；
- planner 使用的 normalized source profile。

Rust ownership 按语义职责拆分，而不是按旧 Swift 目录形状拆分。当前 owner 包括 `xdremux-metadata`、`xdremux-hdr`、`xdremux-container`、`xdremux-motion-photo` 和 `xdremux-classification`。

语义模型应优先表达归一化事实，而不是直接把 vendor field 向上传递。厂商解析可以存在于这一层边缘，但稳定 normalized model 存在时，高层应消费 normalized model。

### 第 3 层：确定性规划与策略

这一层回答：**给定事实、用户请求和可用能力，系统应该做什么？**

Rust owner 是 `xdremux-engine`。

规划必须确定且无副作用。它可以消费：

- normalized source fact；
- user intent；
- product policy；
- capability inventory。

它必须输出显式 plan，并记录 effective choice 和 required operation。

Engine 使用 operation-scoped capability fact，而不是巨型 platform backend。一次转换可以组合多个 adapter 提供的能力。Capability discovery 只报告事实，不负责选择 product policy。

当某个低层 representation type 确实属于稳定契约时，planner 可以引用它；但这不能把 parsing、I/O 或 platform behavior 一起拖进 planning。

不要把策略藏进 codec wrapper、native helper、CLI parser 或 model predictor。

### 第 4 层：执行 Adapter

这一层回答：**哪个具体实现执行某个必需操作？**

Adapter 拥有 side effect、external library 和 platform dependency。例如：

- raster decoding；
- HEVC Gain Map tile encoding；
- RAW processing；
- ImageIO 或 Photos consumer validation；
- Photographic Styles behavior；
- Apple Portrait behavior。

Engine 依赖窄 operation contract。具体 adapter 依赖这些 contract，而不是反过来。

`xdremux-codec` 是第一个具体 Rust adapter boundary，通过 portable libheif provider 实现 engine codec port。crate 已经存在不等于 capability 已经通过 promotion evidence；当前 provider 支持范围和 CI 状态必须从 active Rust branch 实时读取。

不要创建一个把无关能力全部合并起来的万能 `Backend`。也不要为了和旧 Swift 目录一一对应而新建 crate。只有当某项能力具有稳定契约、独立测试并且有独立演化理由时，才建立新边界。

### 第 5 层：Asset 转换与发布

这一层回答：**如何安全地把计划中的媒体结果真正物化？**

职责包括：

- HEIF output construction；
- 需要时的 compressed-sample passthrough；
- Motion Photo → Live Photo；
- Live Photo timing 和 shared asset identity；
- pair publication、provenance、collision handling 和 crash recovery；
- publication 成功之前的 output validation。

Publication 本身就是 correctness 的一部分。一个结构正确的临时输出在 publication contract 满足之前，不是成功的产品结果。

### 第 6 层：产品组合

这一层回答：**用户或应用怎样调用整个系统？**

职责包括：

- CLI parsing 和本地化 terminal output；
- library API composition root；
- batch orchestration；
- macOS app；
- progress 和 structured event；
- product default。

产品 shell 负责把 user intent 翻译成 engine request 和 adapter composition。它们不得重新成为媒体语义的第二 owner。

## 横向控制平面

控制平面不是更高一层产品架构。它横跨所有层。

### Routing metadata

`docs/agent-map.json` 是机器可读 routing index，保存稳定 capability identifier、owner hint、evidence category 和长生命周期 branch role。

该 JSON 不是第二份 architecture specification。人类可读的语义和依赖规则仍以本文档为准。CI 必须保证 routing identifier 与 architecture 同步。

### 验收与一致性

`AGENTS.md`、`scripts/agent_completion_gate.py`、CI、fixture 和 conformance oracle 共同定义某个结论需要什么证据。

必须同时区分 **evidence class** 和 **evidence role**。regression test 与 device test 是不同 evidence class；required merge gate、capability promotion gate 和 diagnostic probe 则是不同 evidence role。

Diagnostic probe 可以临时 patch checkout 或检查环境来发现事实，但在相关行为被固化为可复现 contract check 之前，它不是 acceptance 或 promotion evidence。见[验证 runbook](validation/README.md)。

Rust 代码看起来和 Swift 等价，或者某个诊断 workflow 变绿，都不表示迁移已经完成。只有相关行为契约拥有所需独立证据时，迁移才算完成。

迁移期跨语言对比很有用，但旧实现不会自动成为规范。如果旧实现与外部标准、当前产品契约或更强证据冲突，必须明确解决冲突。

### 研究平面

`Models/`、model card、训练/评估脚本和研究分支组成一个与产品架构并列的研究平面。

研究模型可以输出 **candidate** 或 **proposal**，但不得静默成为产品事实来源。

对于 Photographic Styles，必须把以下边界分开：

1. 输入和 provenance；
2. teacher 或 label source；
3. model prediction；
4. uncertainty 或 gate decision；
5. native consumer 或 renderer evidence；
6. product promotion decision。

更低的 offline loss 本身不足以证明模型可以进入产品默认路径。

稳定 model contract 归入 model card；稳定 training、dataset 和 evaluation protocol 在进入产品前归入研究线的专门研究文档。不要把通用 development guide 变成 experiment log。

### 执行计划

只有工作需要跨一次 Agent session 或一个 PR 时才使用 `docs/exec-plans/`。计划保存可恢复的 fact、decision、evidence、blocker 和 next action。它不能替代 architecture，也不得保存私有 chain-of-thought。

如果 execution plan 发现稳定的仓库级规则，应把规则 promotion 到真正的 normative owner，而不是只留在 completed plan 中。

## 稳定能力词汇

当它们能让范围更清楚时，在 plan、issue、PR 和架构讨论中使用这些 capability identifier。同一组 identifier 会镜像到 `docs/agent-map.json`，用于机器 routing。

| Capability | 架构 owner | 常见证据 |
| --- | --- | --- |
| `format.binary` | 第 1 层 | parser/constructor vector、malformed-input test |
| `format.heif` | 第 1、5 层 | structural conformance、真实输出检查 |
| `metadata.vendor-hdr` | 第 2 层 | fixture extraction、跨实现 vector |
| `hdr.gain-map` | 第 2 层 | formula parity、image/metadata validation |
| `media.motion-photo` | 第 2 层 | vendor fixture、topology/timing test |
| `media.live-photo` | 第 5 层 | pair identity、timing、PhotoKit/device evidence |
| `asset.classification` | 第 2 层 | classification contract fixture |
| `engine.plan` | 第 3 层 | deterministic request/analysis/plan vector |
| `adapter.codec` | 第 4 层 | capability advertisement、payload test、provider round trip |
| `adapter.apple.styles` | 第 4 层 | native consumer/renderer 和 device evidence |
| `adapter.apple.portrait` | 第 4 层 | native consumer 和 device evidence |
| `product.cli` | 第 6 层 | parser、routing、output、exit-contract test |
| `product.app` | 第 6 层 | app integration 和 UI workflow evidence |
| `research.styles-model` | 研究平面 | data provenance、held-out/OOD、consumer A/B |

只有稳定职责无法放进现有 capability 时，才增加新的 identifier。

## 实现线与 Branch 职责

Branch name 是 routing metadata，不是架构。长生命周期 branch role 以 `docs/agent-map.json` 的机器可读定义为准。

### `main`

`main` 是已经发布的 v1.4 产品，也是 Swift + Python 这一代当前公开行为的参考线，同时也是 shared architecture 和 validation contract 的目标归属位置。

v1.4 是最后一个同时发布 Swift 和 Python 实现的 release line。新的产品开发正在转向 Rust。

当已发布行为、安全属性、shared documentation contract 或 migration oracle 需要修正时维护 `main`。不要为了让旧实现和 rewrite 永远保持功能对称，而继续在 `main` 上增加大型新架构。

### `feat/rust-xdremux-format`

这个分支名已经是历史遗留。它现在承载整个 Rust rewrite，而不仅是 format parser。

把它视为主动 migration implementation line。当前 workspace membership、code、test 和 workflow result 直接检查该分支的 `Cargo.toml`、实现和 CI；不要把 roadmap 里的手抄 crate 列表当成权威状态。

把 `main` 和 v1.4 evidence 视为迁移参考输入，而不是逐目录、逐函数照抄清单。

### `codex/reverse-key1-oppo-solver`

这个分支是 Photographic Styles 研究线。

它包含 model training、可选 RAW-linear 和 Gain Map modality、Core ML 工作、provenance control 和 evaluation tooling。该分支的 model card 和 measured artifact 对“该分支研究事实”具有权威性，但不能据此设置 production default。

在[迁移路线图](roadmap.md)中的 promotion gate 全部通过之前，该分支输出都只是 research candidate。

## Source-of-truth 规则

不同问题使用最窄的权威来源：

1. 已发布用户行为：使用当前 release contract、当前公开文档、实现和匹配证据。
2. 稳定架构 ownership：使用本文档。
3. capability routing 和长生命周期 branch role：使用 `docs/agent-map.json`。
4. active branch implementation fact：检查该分支，并与 intended base 比较。
5. 当前 branch/HEAD/divergence：使用 `scripts/agent_context.py` 或 Git 实时推导。
6. behavioral invariant：优先使用外部标准、fixture、conformance test 和 device evidence，而不是 comment 或历史实现形状。
7. research claim：使用 model card、dataset provenance、evaluation code 和当前 measured artifact。
8. 带日期 audit、旧 PR 描述、completed plan 和旧 experiment note 都是 historical evidence，除非当前文档明确采用其结论。

任何长生命周期 branch 都不得成为某个稳定系统契约唯一存在的位置。

## Agent 启动协议

对于较大的任务，在大范围搜索代码之前先建立小 working set：

1. 运行 `python3 scripts/agent_context.py status`。未注册 branch 用 `--base` 显式传 intended base。
2. 确定受影响 capability identifier 和 layer。使用 `python3 scripts/agent_context.py capability <id>` 路由。
3. 只阅读 owning module、相邻测试、相关 fixture 和当前 normative document。
4. 在假设任一侧包含全部最新知识之前，先比较 active branch 与 intended base。
5. roadmap 用于 migration/research state，不用于保存高频 Git 状态。
6. 只有满足 `docs/exec-plans/README.md` 条件时才创建 execution plan。
7. 跨层或改变契约的任务在实现之前先写显式 acceptance criteria。
8. 声称完成之前执行 `AGENTS.md` 的验收流程。

该协议目标是在不降低正确性的前提下减少全仓库扫描和无关上下文消耗。

## 依赖与指令规则

Rust 迁移及之后维护过程中保留这些规则：

- format primitive 不依赖 media policy；
- normalized media semantic 不依赖 CLI、App 或 research code；
- planner 不做 I/O，也不直接调用具体 platform framework；
- adapter 依赖 operation contract，不拥有 product policy；
- publication 不根据文件名形状猜 provenance；
- product shell 不重新实现 parser 或 media semantic；
- research code 不静默设置 production default；
- validation code 可以观察所有层，但不得变成隐藏的 production dependency；
- cross-language oracle 是迁移工具，不是永久架构耦合，除非 release contract 明确要求保留多实现；
- root instruction 只保存 universal rule；
- 只有真实 local invariant 才增加 path-specific 或 nested agent instruction，绝不把仓库级规则复制到多份 instruction file。

当一个修改方案违反这些规则时，优先把职责移动到真正 owner layer，而不是再增加一个 special case。

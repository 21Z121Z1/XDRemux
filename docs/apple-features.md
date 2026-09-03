# Apple 功能

[English](apple-features.en.md) | 简体中文

摄影风格和 Apple 人像已经是 XDRemux 中由 Rust 持有的产品 intent。它们不是第二套产品栈。

Canonical 公开产品是 Rust `xdremux` CLI。Swift target 只作为无法跨平台完成的 Apple framework adapter，例如 ImageIO consumer probing、Vision observation 和 VideoToolbox encoding。

## 当前可用性

标准 HDR、OPPO 兼容输出、Motion Photo → Live Photo、batch、分类、inspect 和 portable validation 已属于 canonical Rust 产品。

摄影风格和 Apple 人像通过 `convert`/`batch` 的产品 intent 表达，不新增 Apple 专用 CLI 子命令，也不把 adapter 或 solver 的底层控制暴露为公开契约。

Rust 持有 feature request/result model、routing、fallback、validation policy、metadata synthesis、assembly 和 publication lifecycle。Apple-native 代码只执行 Rust 请求的平台 operation，并返回事实 observation。

## 平台边界

目标结构是：

```text
xdremux CLI
    ↓
Rust runtime
    ↓
Rust engine policy
    ↓
portable providers + Apple platform adapter
                         ↓
          ImageIO / Core Image / Vision /
          Core ML / AVFoundation / ...
```

`xdremux-apple-adapter` 是由 Rust 产品消费的可分发平台组件。它不是用户 CLI，也不持有产品 policy。

CLI/runtime 当前使用有版本号、生命周期有界的 helper-process protocol。对于 sandboxed macOS App，如果需要独立 entitlement、sandbox、lifecycle 或 crash isolation，可以使用 XPC。transport 刻意保持为 runtime 私有实现，因此更换 transport 不会改变 engine 或 CLI 语义。

## Rust 持有的 policy

Rust 已把用户层 Apple feature intent 建模为两个事实：摄影风格和人像。旧 Swift 的 producer、donor、backend、research control 不作为产品配置重新暴露。

第一项真实 Apple adapter operation 是 ImageIO auxiliary-resource probing。adapter 只报告 observation，例如：

- ISO Gain Map 是否存在；
- disparity 是否存在；
- Portrait Effects Matte 是否存在；
- skin、hair、teeth、glasses semantic matte 是否存在。

adapter 不回答“这是不是有效人像输出”这类业务问题。该判断由 `xdremux-engine` 中的 `AppleImageAuxiliaryFacts` 和 Portrait resource contract 持有。

后续迁移也应沿用同一模式：返回最窄且足够的 framework fact 或 operation result，再由 Rust 持有 policy。

## 摄影风格迁移

Rust 持有 style generation 语义、constrained search、source-bound policy、key1/property-list synthesis、graph assembly、validation policy 和 publication。adapter 只执行 Rust runtime 请求的 framework observation 或 encoding primitive。

研究型 producer、model experiment、donor diagnostic 和 RAW experiment 继续作为 research tooling，不定义公开 CLI 契约，也不构成第二套 runtime。

## Apple 人像迁移

Rust 持有 Portrait preflight、OPPO block parsing、focus/orientation policy、JPEG/container logic、Gain Map policy、REND generation、auxiliary-manifest construction、feature routing、output naming、validation policy 和 atomic publication。ImageIO 只报告 auxiliary-resource facts，adapter 只执行 Rust transaction 要求的 Apple framework operation。

## Live Photo

普通 Motion Photo → Live Photo 已经是 Rust 产品能力，不应绕回 Apple capability adapter。

如果未来组合功能需要对 Live Photo 静态照片执行 Apple-only operation，Rust 仍必须持有 Live Photo asset lifecycle、pair identity、publication 和 validation ordering。Apple adapter 只接收它真正需要执行的窄平台 operation。

## 兼容规则

产品级 compatibility rule 属于 Rust。例如 Apple editing feature 是否能与 OPPO-compatible output 组合、源资产是否具备 Portrait editing 所需资源，以及组合请求如何 atomic publish，都应由 Rust 决策。

不要因为旧 Swift 实现当前负责这些判断，就把这些 policy 固化进 adapter protocol。

## 验证与验收

把证据分成三类：

1. 结构证据证明 HEIF/MOV resource 和 metadata 存在且可解析；
2. 原生 framework 证据证明被测试的 Apple framework 能接受或暴露预期资源；
3. 真机证据证明特定真实设备和 Apple Photos 版本上的行为。

涉及 Apple Photos 交互式编辑的结论，不能用结构证据替代真机证据。

Canonical completion gate 要求 Rust workspace 和真实 Rust → Apple adapter handshake 在 macOS 上通过。feature-specific gate 驱动 Rust CLI 后查询 Apple consumer facts。结构和 native-framework evidence 本身不等于 visual equivalence 或 Photos 真机验收。

## 研究材料

仓库仍保留摄影风格研究代码和 `ReverseKey1Ensemble` 等可选模型。它们属于研究/训练资产，不是产品模式。需要时见[模型卡](../Models/ReverseKey1Ensemble.model-card.md)。

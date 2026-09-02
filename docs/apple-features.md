# Apple 功能

[English](apple-features.en.md) | 简体中文

摄影风格和 Apple 人像是 XDRemux 当前剩余最大的 Apple 特有所有权迁移块。它们不是第二套产品栈。

Canonical 公开产品是 Rust `xdremux` CLI。旧 Swift Apple 实现只作为迁移 oracle，以及目前仍需要 ImageIO、Core Image、Vision、Core ML、AVFoundation 等 Apple framework 的平台能力实现继续存在。

## 当前可用性

标准 HDR、OPPO 兼容输出、Motion Photo → Live Photo、batch、分类、inspect 和 portable validation 已属于 canonical Rust 产品。

摄影风格和 Apple 人像目前还没有作为稳定命令/参数暴露在 canonical Rust CLI 中。不要把 legacy Swift-only 参数当成新的产品规范。

只有在 Rust 持有 feature request/result model、routing、fallback、validation policy 和 publication lifecycle，而 Apple-native 代码只执行 Rust 请求的平台 operation 时，Apple feature ownership migration 才算完成。

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

旧 Swift 实现仍包含大量 style generation 和 Apple-framework 行为，包括 semantic analysis、Core ML 路径、constrained style-data generation 和 Apple-specific consumer validation。

不要把它现有的命令行参数或内部 producer selection 机械移植进 Rust。应把实现拆成窄平台 operation，例如 framework analysis 或 model execution，而 feature routing 和产品默认值由 Rust 持有。

研究型 producer、model experiment、donor diagnostic 和 RAW experiment 继续作为 research tooling，不定义公开 CLI 契约。

## Apple 人像迁移

旧 Swift Portrait pipeline 仍执行 Apple-framework decoding/writing，并且还包含正在从 Swift 移出的 policy。

第一块 policy 已开始转移到 Rust：ImageIO 只报告 auxiliary-resource facts，由 Rust 判断完整 resource set 是否满足 Portrait editing contract。

后续继续按同一方向迁移。OPPO block parsing、JPEG/container logic、Gain Map policy、feature routing、output naming 和 validation policy 属于 Rust；只有确实需要 Apple framework 的 operation 留在 Apple capability layer。

## Live Photo

普通 Motion Photo → Live Photo 已经是 Rust 产品能力，不应重新绕回 legacy Swift Apple feature engine。

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

Canonical completion gate 要求 Rust workspace 和真实 Rust → Apple adapter handshake 在 macOS 上通过。每迁移一个 Apple operation，应补对应 replacement gate。只有 replacement evidence 完成后，才删除相应 legacy Swift 实现。

## 研究材料

仓库仍保留摄影风格研究代码和 `ReverseKey1Ensemble` 等可选模型。它们属于研究/训练资产，不是产品模式。需要时见[模型卡](../Models/ReverseKey1Ensemble.model-card.md)。

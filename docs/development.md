# 开发与构建

[English](development.en.md) | 简体中文

维护已经发布的 v1.4 Swift/Python 线、集成 Swift package 或构建 macOS App 时使用本文档。

v1.4 之后的新产品开发正在转向 Rust rewrite。跨模块工作之前先读[系统架构](architecture.md)，迁移工作之前先读[迁移路线图](roadmap.md)。不要根据下面的 v1.4 目录布局推断未来架构。

命令行用法见 [CLI 参考](cli.md)。

## Release 与开发线

`v1.4` 是最后一个同时发布 Swift 和 Python 实现的 release。

已发布 Swift/Python 行为以 v1.4 release 和当前 `main` 维护线为参考。主动迁移实现位于 Rust rewrite branch；开始工作前先把它与 intended base 比较。

编程语言本身不是架构边界。稳定 ownership 由 `architecture.md` 中的 capability 和 layer model 定义。

## 工具链

v1.4 Swift package manifest 当前设置：

- Swift tools version 6.0；
- 最低平台 macOS 15；
- package default localization 为 `en`。

当前 targets 使用 Swift language mode 5。

构建和测试 Swift/Python 线：

```bash
swift build
swift test
python3 -m unittest discover -s Tests -v
```

Rust migration command 和 crate-specific gate 位于 active Rust branch。不要仅为了同步 branch-local implementation state 就把 Rust 命令复制到这份 v1.4 指南；稳定 migration contract 应 promotion 到 architecture 或 roadmap。

## Swift package 产品

| 产品 | 类型 | 用途 |
| --- | --- | --- |
| `XDRemuxCore` | library | HDR 转换、HEIF/ISO-BMFF、metadata、Motion Photo 解析、分类和共享验证。 |
| `XDRemuxAppleFeatures` | library | Apple Live Photo、摄影风格、Apple 人像和 Apple 特有分析。 |
| `xdremux` | executable | Swift 命令行界面。 |

`CoreImageRAWDiagnostics` 是开发者诊断 target，不是公开 package product。

构建：

```bash
swift build --target CoreImageRAWDiagnostics
```

## Package 集成

集成已发布实现时，应使用适合该项目的 release version 或 exact revision。依赖 `main` 会跟随维护分支，并可能收到 API 变化。

仓库中的 v1.4 source 仍使用下面描述的 package product。

标准转换链路使用 `XDRemuxCore`。

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
let request = ConversionRequest(
    input: input,
    output: OutputTarget.file(outputURL).destination(for: input),
    configuration: ConversionConfiguration()
)

let result = try ConversionEngine.convert(request)
```

Apple 特有转换引擎使用 `XDRemuxAppleFeatures`。

## 仓库结构

下面的布局描述 v1.4 Swift/Python 实现。它是 implementation map，不是系统架构。

| 路径 | 用途 |
| --- | --- |
| `Sources/XDRemuxCore/` | 核心转换和格式逻辑。 |
| `Sources/XDRemuxAppleFeatures/` | Apple 特有转换和验证。 |
| `Sources/XDRemuxCLI/` | Swift CLI 参数解析和命令路由。 |
| `Sources/CoreImageRAWDiagnostics/` | 开发者 RAW 诊断 target。 |
| `xdremux_py/` | 跨平台 Python 实现。 |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI App。 |
| `Tests/` | Swift 测试、Python policy test 和验证 harness。 |
| `fixtures/` | strict CI gate 使用的版本化真实 Motion Photo fixture。 |
| `scripts/` | 构建、评估和验收工具。 |
| `docs/` | 当前文档和历史验证记录。 |
| `Models/` | 可选研究模型和模型文档。 |

代码迁移到 Rust 时按 capability contract 和 evidence 映射行为。不要把这些目录一一复制成 crate。

## Apple 辅助进程

部分 Apple 功能会编译或运行 package resource 中的 helper program。

helper toolchain 根据源码内容生成 hash，并缓存兼容的构建结果。实现会限制 helper 调用时长，并在协议要求时把机器可读 stdout 和诊断输出分离。

Apple 私有 API 兼容性必须在运行时检查。runtime method signature 不符合已支持 ABI 时，不要按假定 ABI 调用私有 Objective-C selector。

当前 macOS 27 摄影风格响应 helper 会在调用前检查已知 initializer 和 style-apply ABI 形状。

这些 helper 是 v1.4 的 execution mechanism。在 Rust 架构中，Apple-only behavior 应位于显式 operation-scoped adapter capability 后面，而不是进入纯 semantic engine。

## macOS App

App 位于 `apps/macos/XDRemuxApp/`。

常用命令：

```bash
scripts/build_and_run.sh run
scripts/build_and_run.sh build
scripts/build_and_run.sh debug
scripts/build_and_run.sh verify
scripts/build_and_run.sh logs
scripts/build_and_run.sh clean
```

App 直接链接 Swift package，不把 CLI 当作核心转换子进程。

如果 Rust transition 之后 App 继续使用 SwiftUI，应通过窄的 library 或 FFI composition boundary 集成 Rust。不要把 media policy 移到 UI layer。

## Python package

Python v1.4 需要 Python 3.11 或更高版本。

运行时依赖包括：

- `pillow-heif`；
- `Pillow`；
- `numpy`；
- `piexif`。

可选 `training` dependency 会加入 PyTorch。

安装后的命令是 `xdremux-py`。仓库本地入口是 `python3 -m xdremux_py`。

Python 实现继续作为 v1.4 released reference；当其行为有独立 product contract 或 evidence 支持时，也可以作为有价值的 migration oracle。不要仅为了保留 migration parity 就让 Python 成为永久 Rust runtime dependency。

## 调试和研究控制

仓库存在用于编码诊断、scratch 保留、style rendering 和研究模型选择的环境变量。

除非产品行为需要，不要把研究环境变量加入普通用户命令。

如果测试和当前产品链路都不依赖某个研究开关，不要把它写成稳定公开接口。

可选 Reverse Key 1 模型有独立[模型卡](../Models/ReverseKey1Ensemble.model-card.md)。

Research model output 是 candidate，不是 product policy。进入未来 Rust product capability 之前，必须满足[迁移路线图](roadmap.md)中的 research promotion gate。

## Completion gate

仓库 Agent 在声称工作完成前，必须验证准确的已提交 `HEAD`。

操作契约见 [AGENTS.md](../AGENTS.zh-CN.md)。验收 runbook 见 [validation/README.md](validation/README.md)。

默认使用与修改范围匹配的验证。纯文档修改不需要完整真实照片矩阵。转换核心修改除了静态检查，还需要 functional evidence。

completion receipt 会绑定 commit、base commit、changed path 和 clean worktree。之后的 tracked edit 会让 receipt 失效。

跨模块 architecture 或 migration work 还必须明确受影响 capability identifier、owning layer、oracle/evidence 和 residual gap。

## 文档修改

当前技术文档遵循[技术写作规范](style-guide.md)。

代码修改改变已记录的命令、输出规则、格式契约、架构边界或验收规则时，先更新英文文档，再更新对应中文翻译。

不要让稳定系统规则只存在于长生命周期 branch、PR 描述或聊天记录。应把它 promotion 到当前 architecture、roadmap、model card、test contract 或其他合适的 normative document。

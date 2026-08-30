# 开发与构建

[English](development.en.md) | 简体中文

修改 XDRemux、集成 Swift package 或构建 macOS App 时使用本文档。

命令行用法见 [CLI 参考](cli.md)。

## 工具链

Package manifest 当前设置：

- Swift tools version 6.0；
- 最低平台 macOS 15；
- package 默认 localization 为 `en`。

当前 targets 使用 Swift language mode 5。

构建和测试：

```bash
swift build
swift test
python3 -m unittest discover -s Tests -v
```

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

把仓库加入 package dependency：

```swift
dependencies: [
    .package(
        url: "https://github.com/21Z121Z1/XDRemux.git",
        branch: "main"
    )
]
```

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

当前 package 文档没有稳定 release tag 契约。依赖 `main` 可能收到 API 变化。

## 仓库结构

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

## Apple 辅助进程

部分 Apple 功能会编译或运行 package resource 中的 helper program。

helper toolchain 根据源码内容生成 hash，并缓存兼容的构建结果。实现会限制 helper 调用时长，并在协议要求时把机器可读 stdout 和诊断输出分离。

Apple 私有 API 兼容性必须在运行时检查。runtime method signature 不符合已支持 ABI 时，不要按假定 ABI 调用私有 Objective-C selector。

当前 macOS 27 摄影风格响应 helper 会在调用前检查已知 initializer 和 style-apply ABI 形状。

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

## Python package

Python package 需要 Python 3.11 或更高版本。

运行时依赖包括：

- `pillow-heif`；
- `Pillow`；
- `numpy`；
- `piexif`。

可选 `training` dependency 会加入 PyTorch。

安装后的命令是 `xdremux-py`。仓库本地入口是 `python3 -m xdremux_py`。

## 调试和研究控制

仓库存在用于编码诊断、scratch 保留、style rendering 和研究模型选择的环境变量。

除非产品行为需要，不要把研究环境变量加入普通用户命令。

如果测试和当前产品链路都不依赖某个研究开关，不要把它写成稳定公开接口。

可选 Reverse Key 1 模型有独立[模型卡](../Models/ReverseKey1Ensemble.model-card.md)。

## Completion gate

仓库 Agent 在声称工作完成前，必须验证准确的已提交 `HEAD`。

验收 runbook 见 [validation/README.md](validation/README.md)。

默认使用与修改范围匹配的验证。纯文档修改不需要完整真实照片矩阵。转换核心修改除了静态检查，还需要 functional evidence。

completion receipt 会绑定 commit、base commit、changed path 和 clean worktree。之后的 tracked edit 会让 receipt 失效。

## 文档修改

当前技术文档遵循[技术写作规范](style-guide.md)。

代码修改改变已记录的命令、输出规则、格式契约或验收边界时，先更新英文文档，再更新对应中文翻译。

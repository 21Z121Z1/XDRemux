# XDRemux 开发与构建

[English](development.en.md) | 简体中文

本文档面向需要构建 App、集成 Swift Package、运行 validator 或修改转换器的开发者。普通转换用法见 [CLI 参考](cli.md)。

## 开发环境

- macOS 15 或更高版本。
- Swift 6 工具链。
- 构建使用 Apple 最新 SwiftUI API 的 macOS App 时，建议使用当前 Xcode。
- Apple 人像开发需要 `zstd`；JPEG 人像桥接需要 `ultrahdr_app`。

## Swift Package 产品

| Product | 类型 | 用途 |
| --- | --- | --- |
| `XDRemuxCore` | Library | 转换模型、HDR、HEIF、Metadata、Batch 和输出验证 |
| `XDRemuxAppleFeatures` | Library | Apple 语义分析、摄影风格和人像功能 |
| `xdremux` | Executable | 正式用户 CLI |
| `xdremux-dev` | Executable | 实验参数、validator 和诊断命令 |
| `XDRemuxSemanticHelper` | Executable | 隔离的 Apple 语义分析 |
| `XDRemuxHEVCEncoderHelper` | Executable | 隔离的 VideoToolbox HEVC 编码 |
| `XDRemuxStyleValidationHelper` | Executable | 隔离的 Apple 摄影风格属性验证 |

基础命令：

```bash
swift build
swift test
swift run xdremux --help
swift run xdremux-dev --help
```

使用 Apple 功能时不要只构建单个 CLI product；完整 `swift build` 会同时生成所需 helper。

## Swift Package 集成

```swift
dependencies: [
    .package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
]
```

按需要依赖 `XDRemuxCore` 或 `XDRemuxAppleFeatures`。基础转换入口：

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
var configuration = ConversionConfiguration()
configuration.eventHandler = { event in
    // 将结构化事件映射到调用方的日志或 UI。
}

let cancellation = ConversionCancellation()
configuration.cancellation = cancellation

let request = ConversionRequest(
    input: input,
    output: OutputTarget.file(outputURL).destination(for: input),
    configuration: configuration
)
let result = try ConversionEngine.convert(request)
```

Apple 功能通过 `configuration.appleFeatureOptions` 配置，并调用 `AppleFeatureConversionEngine.convert(_:)`。

`XDRemuxCore` 不负责终端、ANSI、本地化、SwiftUI 或 GitHub Actions 输出。调用方通过 `ConversionEvent` 获取阶段、warning、completed 和 failed 事件，并通过 `ConversionCancellation` 取消任务。

在发布稳定 tag 前，外部项目跟随 `main` 需要自行承担 API 变化；发布 tag 后应改用语义版本范围。

## 预构建 helper

Apple 私有或需要进程隔离的流程使用正式 executable target。helper 在构建时生成，App 将其放入 `Contents/Helpers`；运行时不会搜索源码、计算 source hash、调用 `xcrun`、`swiftc` 或 `clang`。

当前协议标识：

- `xdremux-semantic-helper-v1`
- `xdremux-hevc-encoder-helper-v1`
- `xdremux-apple-semantic-style-properties-probe-v1`

helper stdout 只输出版本化机器协议，stderr 只输出诊断。App 和 CLI 使用同一个 locator，并支持超时和取消。

## 开发者 CLI

内部选项只在 `xdremux-dev` 提供：

```bash
swift run xdremux-dev convert \
  --input IMG_001.heic \
  --family x7 \
  --input-processing hybrid \
  --oppo-compat auto \
  --oppo-camera-tail preserve \
  --tmap-format imageio \
  --diagnostics-dir diagnostics/
```

保留的内部参数包括 `--family`、`--input-processing`、`--oppo-compat`、`--oppo-camera-tail`、`--tmap-format` 和 `--diagnostics-dir`。正式 `xdremux` 会拒绝这些参数。

验证命令：

```bash
swift run xdremux-dev validate-apple --input output.heic
swift run xdremux-dev validate-portrait --input output.heic --json validation.json
swift run xdremux-dev portrait-self-test
```

## macOS App

App 位于 `apps/macos/XDRemuxApp/`，直接链接共享 Swift Package，不通过完整 CLI 执行转换。

```bash
scripts/build_and_run.sh build
scripts/build_and_run.sh run
scripts/build_and_run.sh verify
scripts/build_and_run.sh debug
scripts/build_and_run.sh logs
scripts/build_and_run.sh logs --all
scripts/build_and_run.sh clean
```

`build` 只构建；`run` 构建并启动；`verify` 还检查 bundle、helper 签名和进程；`debug` 使用 LLDB。`logs` 只显示 `com.proxdr.XDRemuxApp` subsystem，`logs --all` 才显示该进程的完整系统日志。

默认使用 quiet `xcodebuild`。完整 `build.log` 和 `.xcresult` 保存在 XDRemux 自己的 DerivedData 中，`--verbose` 才实时输出完整构建日志。

## 仓库结构

| 路径 | 用途 |
| --- | --- |
| `Package.swift` | SwiftPM 产品和 target 定义 |
| `Sources/XDRemuxCore/` | 平台无关转换核心 |
| `Sources/XDRemuxAppleFeatures/` | Apple 专用功能 |
| `Sources/XDRemuxCLI/` | CLI 解析、本地化和输出 |
| `Sources/XDRemuxExecutable/` | 正式 CLI 入口 |
| `Sources/XDRemuxDevExecutable/` | 开发者 CLI 入口 |
| `Sources/XDRemux*Helper/` | 构建时生成的隔离 helper |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI App |
| `Tests/` | Swift、Python 和验证测试 |
| `scripts/` | 构建、验证和 projection 工具 |
| `docs/` | 用户文档、工程文档、验证和研究资料 |

## 验收规则

所有完成声明都需要与最终提交绑定的 completion gate receipt，但 gate 必须按变更范围选择证据。

- 文档变更：链接、命令示例、文档结构和公开 projection 检查。
- CLI 解析变更：对应参数与输出回归测试。
- 转换核心变更：单元测试加真实功能或集成验证。
- App/helper 变更：构建、签名、运行或设备证据。
- 发布或跨模块变更：完整矩阵。

不要因为 completion gate 存在就对每次 README 修改运行真实照片矩阵。先完成并提交同一批相关改动，再为最终 `HEAD` 运行一次 targeted plan：

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

详细 schema 与证据要求见[验证说明](validation/README.md)。

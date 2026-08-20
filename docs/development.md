# XDRemux 开发与构建

[English](development.en.md) | 简体中文

面向要改转换器、集成 Swift Package 或构建 macOS App 的人。日常转换用法见 [CLI 参考](cli.md)。

## 环境

- macOS 15 或更高版本
- Swift 6 工具链
- 构建 macOS App 需要 Xcode
- Apple 人像功能需要 `zstd`（`brew install zstd`）

## Swift Package 产品

| 产品 | 类型 | 用途 |
| --- | --- | --- |
| `XDRemuxCore` | Library | 转换核心：HDR、HEIF、元数据、批量、输出校验 |
| `XDRemuxAppleFeatures` | Library | Apple 语义分析、摄影风格和人像 |
| `xdremux` | Executable | 命令行工具 |

```bash
swift build
swift test
swift run xdremux --help
```

RAW 的 CoreImage 探测程序仍保留为开发 target，但不再作为对外 Swift Package 产品发布。需要时显式构建：

```bash
swift build --target CoreImageRAWDiagnostics
```

它的入口源码在 `Sources/CoreImageRAWDiagnostics/main.swift`，参数契约是 `DNG_DIRECTORY OUTPUT_DIRECTORY [MAX_SIZE]`。

## 集成到自己的项目

```swift
dependencies: [
    .package(url: "https://github.com/21Z121Z1/XDRemux.git", branch: "main")
]
```

按需依赖 `XDRemuxCore` 或 `XDRemuxAppleFeatures`：

```swift
import XDRemuxCore

let input = InputSource(url: inputURL)
var configuration = ConversionConfiguration()
configuration.eventHandler = { event in
    // 把结构化事件接到调用方的日志或界面上。
}

let result = try ConversionEngine.convert(
    ConversionRequest(
        input: input,
        output: OutputTarget.file(outputURL).destination(for: input),
        configuration: configuration
    )
)
```

Apple 功能通过 `configuration.appleFeatureOptions` 配置，入口是 `AppleFeatureConversionEngine`。

`XDRemuxCore` 不碰终端、ANSI、本地化、SwiftUI 和 CI 输出 —— 调用方通过 `ConversionEvent` 拿阶段、warning 和结果。

目前还没有发布稳定 tag，跟 `main` 意味着 API 可能变。

## 运行时编译的 helper

Apple 功能里的 Vision 分析、HEVC 编码和风格属性探测跑在独立进程里。这些 helper 的源码放在包资源里，**第一次用到时才编译**：`AppleNativeToolchain` 按源码内容算哈希，调 `/usr/bin/xcrun` 编译到用户缓存目录，之后同一份源码直接复用缓存。

这带来两个后果：

- 首次运行 Apple 功能会有一次编译等待，之后就没有了。
- 缓存目录是全机共享的，所以编译产物用临时文件加原子改名发布，多个 XDRemux 进程同时首次运行不会读到写了一半的二进制。

所有 helper 调用都有超时；stdout 只走版本化的机器协议，诊断走 stderr。

## macOS App

App 在 `apps/macos/XDRemuxApp/`，直接链接 Swift Package，不通过子进程调 CLI（`Tests/test_swift_app_architecture.py` 会强制这一点）。

```bash
scripts/build_and_run.sh run      # 构建并启动
scripts/build_and_run.sh build    # 只构建
scripts/build_and_run.sh debug    # Debug 配置构建
scripts/build_and_run.sh verify   # swift build + swift test + Python 套件
scripts/build_and_run.sh logs     # 显示上次构建日志
scripts/build_and_run.sh clean    # 清掉 DerivedData
```

除显式 `debug` 外一律 Release 构建 —— 摄影风格求解很吃 CPU，调试构建慢好几倍。

## 仓库结构

| 路径 | 用途 |
| --- | --- |
| `Package.swift` | SwiftPM 产品和 target 定义 |
| `Sources/XDRemuxCore/` | 转换核心 |
| `Sources/XDRemuxAppleFeatures/` | Apple 专用功能 |
| `Sources/XDRemuxCLI/` | 命令行解析与入口 |
| `Sources/CoreImageRAWDiagnostics/` | 仅开发使用的 RAW 诊断 target |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI App |
| `xdremux_py/` | 跨平台 Python CLI 实现与仓库内模块入口 |
| `Tests/` | Swift 测试、Python 策略套件和验证 harness |
| `scripts/` | 构建与验收脚本 |

Swift CLI 只维护 SwiftPM 的 `xdremux` executable；仓库内部验证也直接构建和调用这个产品，不再维护独立的单文件 Swift 转发入口。Python CLI 同样只维护 `xdremux-py` 和 `python3 -m xdremux_py` 两个由同一个包提供的入口。

## 调试用环境变量

正常使用都不需要设置。

| 变量 | 作用 |
| --- | --- |
| `XDREMUX_DISABLE_DIRECT_GAIN=1` | 关掉一次性直接编码 Gain Map 的快路径 |
| `XDREMUX_KEEP_GAIN_SCRATCH=1` | 保留 Gain Map 中间产物 |
| `XDREMUX_KEEP_PORTRAIT_SCRATCH=1` | 保留人像转换中间产物 |
| `XDREMUX_ENCODING_AUDIT_DIR=<目录>` | 把编码审计数据写到指定目录 |
| `XDREMUX_STYLE_RENDER_JOBS=<n>` | 限制摄影风格渲染并发数 |
| `XDREMUX_RESEARCH_REVERSE_KEY1_COREML_MODEL=<路径>` | 研究用：加载外部 `.mlmodelc` / `.mlpackage` ReverseKey1Net，使用 10 秒有界语义代理选择模型 key1；失败或超时直接回退 identity，不进入慢 solver |
| `XDREMUX_RESEARCH_UNIVERSAL_STYLE_COREML_MODEL=<路径>` | 研究用：单张主图直接预测 key1 和完整候选状态；用 iPhone calibration p95 不确定度和 10 秒语义代理双重门控，失败回退 identity，不先渲染 disabled anchor |

摄影风格还有若干 `XDREMUX_RESEARCH_*` 和 `XDREMUX_STYLES_*` 研究开关，会在输出 manifest 里标记为研究模式并排除生产判定，见 [Apple 功能文档](apple-features.md)。

备选方案使用的 Core ML 模型随仓库放在
`Models/ReverseKey1Ensemble.mlpackage`，模型契约、哈希和验证边界见同目录的
`ReverseKey1Ensemble.model-card.md`。`scripts/export_reverse_key1_coreml.py` 可以从两个训练
checkpoint 重新导出融合模型；训练 checkpoint 和私有样片不进入 Git。
`computeUnits = .all` 允许系统选择 CPU、GPU 或 Neural Engine，但不能据此声称实际落在
Neural Engine。在线语义代理只是完整 Neutrino 响应的快速筛选器，完整 renderer A/B 和
真实 Photos 验收仍是独立证据层。

仓库还包含一个更通用但仍为备选研究用途的单图状态模型：
`Models/UniversalPhotographicStyleStateNet.mlpackage`。它不需要运行时 unstyled 图，
可从 JPEG、HEIC、PNG、TIFF、WebP、AVIF，以及使用嵌入预览的 DNG 构造 key1、
GTC、c/d 和 capture scalars 候选。输入/输出契约、iPhone 会话留出、213 张 OPPO
label-free 覆盖结果和 Core ML 哈希见
`Models/UniversalPhotographicStyleStateNet.model-card.md`。研究开关当前只授权 key1 进入
有界语义代理；GTC、c/d 和 scalars 仅写入调试候选目录，不覆盖最终容器。它没有接入
默认 converter，也没有完成 Photos consumer 验收。

历史 OPPO consumer A/B 产物可用
`scripts/evaluate_oppo_solver_ab.py` 汇总；该脚本会分别保留 direct model proposal、
bounded seeded residual、full solver 与 identity 的 neutral RMSE，并记录 solver/response
artifact hash。当前四个缓存场景都已有调参产物，因此报告会明确标记 `lockedSet` 不可用，
不能把它们当作 untouched 泛化验收。

Universal 训练支持 `--resume`（要求 checkpoint 与 manifest hash 一致）及可选的
`--consumer-weight`。后者只启用 identity-neutral 的十项 encoded-RGB quadratic
consumer proxy 训练正则；它不是 Apple renderer。短程 v3 resume ablation 的 held-out
key1 MAE 有改善，但 proxy 像素 RMSE 仍高于 identity，未导出或替换发布模型。

Privileged-teacher cascade 评测入口为
`scripts/evaluate_universal_paired_cascade.py`。它固定 calibration-derived 的
paired ensemble alpha `0.625`，分别报告 Universal direct、真实 disabled unstyled 的
paired oracle、Universal predicted unstyled cascade、primary-as-unstyled 和 shuffled
controls，并按设备/session 输出结果。当前 held-out cascade 为 `0.82674`，差于 direct
Universal `0.82233`；真实 unstyled oracle 为 `0.78123`，说明主要缺口在 Universal
unstyled estimator。该 cascade 仍是离线诊断，不接 runtime。

## 验收规则

### Native style plist contract selection

Some native iPhone files retain an older `tag:apple.com,2023:photo:metadata:styles`
URI item (plist version 8) alongside the generated Photographic Styles item
(plist version 15). The research converter keeps the native item and adds its
validated style graph; final validation therefore enumerates URI candidates and
selects the byte-identical payload already accepted by the producer. It must not
blindly validate the first URI item. This is a contract-selection fix only; it
does not establish Photos import/edit/save/reopen acceptance.

### OPPO self-pair prospective locked set

`.codex/reverse-key1/self-pair-locked-v1/manifest.json` freezes one existing
source per canonical OPPO model using only file existence, historical
solver-scene/hash exclusion, model grouping, and lexicographically smallest
source SHA-256. The five rows were observed by the label-free OOD inventory,
but were not used for iPhone training or self-pair/solver/consumer tuning.
They therefore measure prospective zero-shot consumer behavior, not entirely
unseen OOD generalization. Consumer results belong in the ignored `.codex`
report and must keep identity, direct self-pair, one-step bounded correction,
and full solver separate; a renderer or target failure remains partial.

For reproducible preliminary-fixture experiments, the constrained solver also
accepts the research-only `XDREMUX_STYLE_SOLVER_MODEL_SEED` pointing to a
native-layout 51,840-byte Float16 key1. `...MODEL_SEED_MODE=fast` records the
bounded direct path; `...=seeded` invokes the one-step native residual path.
Both operate only after the pipeline has generated and validated its own
`constrained-solver-preliminary-identity.heic`.

The native-consumer calibration freeze is stored under
`.codex/reverse-key1/native-consumer-v1/`. It contains four calibration and
four held-out iPhone sessions, selected per device from the existing fixed
dataset split by source SHA only. The first iPhone 16 causal probe proved that
identity and self-pair key1 produce distinct native apply rasters; full HEIC
replacement was not claimed when the current converter rejected the source's
binary-plist contract.
Use `scripts/summarize_native_consumer_calibration.py` to summarize this cache.
It preserves missing conversions and responses as null and refuses to freeze a
choice when calibration coverage is incomplete; it never substitutes identity
or another split for missing native renders.
The pre-registered alpha consumer grid is summarized by
`scripts/summarize_native_alpha_grid.py`; its selection rule is calibration-only
and requires no more than a 10% device regression, no increase in reversals or
failures, and at least 1% aggregate improvement before changing `.625`.

声明改动完成之前，要为最终提交跑一次 completion gate：

```bash
python3 scripts/agent_completion_gate.py run \
  --base origin/main \
  --plan /tmp/xdremux-agent-verification.json

python3 scripts/agent_completion_gate.py verify \
  .codex/verification-receipts/$(git rev-parse HEAD).json
```

receipt 绑定当前 HEAD、base、改动文件集合和干净的工作区，之后任何提交或改动都会让它失效。

按改动范围选证据，不要每改一行文档就跑整套真实照片矩阵：

- 只改文档：链接、命令示例和文档策略检查。
- 改 CLI 解析：对应的参数和输出回归。
- 改转换核心：单元测试加上真实样本的功能验证。
- 改 App 或 helper：构建、运行或设备证据。

计划文件的 schema 和证据要求见[验证说明](validation/README.md)，可复用的 harness 在 `Tests/validation/`。

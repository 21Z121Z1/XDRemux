# 开发与构建

[English](development.en.md) | 简体中文

XDRemux 只有一个产品核心：Rust workspace。新的产品行为必须进入 Rust。Swift 只作为迁移期 Apple 平台能力层，Python 只作为迁移验证和研究工具，不再构成第二套 XDRemux runtime。

面向用户的命令行行为见 [CLI 参考](cli.md)。

## Canonical 开发流程

产品改动使用 Rust workspace：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xdremux-cli -- --help
```

Canonical 产品栈为：

```text
xdremux-cli
    ↓
xdremux-runtime
    ↓
xdremux-engine
    ↓
source / classification
motion-photo / hdr / metadata
container / heif / codec / format
    ↓
portable providers + platform adapters
```

产品意图应停留在这座抽象塔的上层。除非某个 codec、camera-tail、源代际、routing 或 container 选择对应明确不同的用户结果，否则不要把它暴露成用户选项。

## Rust workspace 所有权

| Crate | 职责 |
| --- | --- |
| `xdremux-cli` | 唯一公开 CLI 和命令契约。 |
| `xdremux-runtime` | 文件系统执行、publication、批处理可靠性、恢复以及平台能力协调。 |
| `xdremux-engine` | 产品意图、转换规划、能力要求和 orchestration。 |
| `xdremux-source` / `xdremux-classification` | 输入探测、资产身份和分类。 |
| `xdremux-motion-photo` | Motion Photo 解析和 Live Photo 媒体语义。 |
| `xdremux-hdr` / `xdremux-metadata` | HDR / Gain Map 数学和 metadata primitive。 |
| `xdremux-container` / `xdremux-heif` / `xdremux-codec` / `xdremux-format` | Container、HEIF、codec、JPEG/EXIF/TIFF/ISOBMFF primitive。 |

下层 crate 可以提供格式 primitive，但这不意味着它成为一个产品模式。是否使用该 primitive 由 runtime 和 engine 决定。

## Apple 平台能力

`Sources/XDRemuxAppleFeatures/` 在摄影风格、人像、RAW、Vision、Core ML、AVFoundation 等 Apple framework 能力迁入 Rust capability model 期间继续保留。

目标架构是一个窄平台 adapter：

- Rust 持有 request、result、policy、routing、命名、分类、HDR 行为、Motion Photo 行为、batch 行为和 fallback 决策；
- Apple 层只调用 Apple framework 并返回 capability result；
- 不要再向 Swift 添加跨平台产品 policy。

需要进程内 Rust/Swift interop 时，优先使用稳定 C ABI。FFI 所需的 unsafe 代码应隔离在边界层，不要进入安全的 engine/runtime crate。

`CoreImageRAWDiagnostics` 继续作为开发者专用 Swift target：

```bash
swift build --target CoreImageRAWDiagnostics
```

Swift tests 仍可用于 Apple capability 验收和迁移 oracle，但它们不再定义 canonical CLI 契约。

## Python 工具

Python package 需要 Python 3.11 或更高版本，仅保留用于迁移期 conformance、真实 fixture oracle 和研究/训练流程。它不再安装 CLI，也不得定义新的产品语义。

只有 Python oracle 或研究流程需要时才安装：

```bash
python -m pip install -e .
python -m unittest discover -s Tests -v
```

当前依赖包括 `pillow-heif`、`Pillow`、`numpy` 和 `piexif`。可选 `training` dependency 会加入 PyTorch。

训练和评估脚本可以继续使用 Python，因为它们属于研究工具，而不是第二套 XDRemux 实现。

## 仓库结构

| 路径 | 用途 |
| --- | --- |
| `crates/` | Canonical Rust 产品栈。 |
| `Sources/XDRemuxAppleFeatures/` | Apple capability 实现和迁移期验证。 |
| `Sources/XDRemuxCore/` | 仅在 replacement evidence 尚未完成时保留的 legacy Swift core。 |
| `Sources/XDRemuxCLI/` | 仅在 Apple 迁移工作仍有引用时保留的 legacy Swift CLI。 |
| `Sources/CoreImageRAWDiagnostics/` | 开发者 RAW 诊断。 |
| `xdremux_py/` | 迁移 oracle 和研究/训练工具；没有产品 CLI。 |
| `apps/macos/XDRemuxApp/` | 产品栈迁移期间的 macOS SwiftUI App。 |
| `Tests/` | Rust/Swift/Python 验收测试和 validation harness。 |
| `fixtures/` | strict gate 使用的版本化真实媒体 fixture。 |
| `scripts/` | 构建、评估、迁移和验收工具。 |
| `docs/` | 当前指导文档和历史研究记录。 |
| `Models/` | 可选研究模型和模型文档。 |

Legacy Swift/Python 代码今后只应接受迁移、conformance、安全修复或删除工作。新的用户可见行为进入 Rust。

## Apple 辅助进程

部分 Apple 功能会编译或运行 package resource 中的 helper program。

helper toolchain 根据源码内容生成 hash，并缓存兼容构建结果。helper 调用应保持有界；协议需要时，机器可读 stdout 必须与诊断输出分离。

Apple 私有 API 兼容性必须在运行时检查。runtime method signature 不符合已支持 ABI 时，不要按假定 ABI 调用私有 Objective-C selector。

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

迁移期间 App 仍可能链接 Swift package。不要把这一依赖解释为 Swift 继续拥有 canonical conversion policy；可复用的产品行为应优先迁入 Rust。

## 调试和研究控制

仓库存在用于编码诊断、scratch 保留、style rendering 和研究模型选择的环境变量。

除非产品行为确实需要，不要把研究环境变量加入普通用户命令。如果 canonical Rust 产品路径和测试都不依赖某个研究开关，就不要把它写成稳定公开接口。

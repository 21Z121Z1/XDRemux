# 开发与构建

[English](development.en.md) | 简体中文

XDRemux 只有一个产品核心：Rust workspace。唯一公开 CLI 是 Rust `xdremux` 二进制程序。新的产品行为由 Rust 实现。Swift 及其原生目标负责 Apple 框架操作，Python 负责研究和训练工具。

本文说明组件职责、构建命令和打包边界。面向用户的命令行为见 [CLI 参考](cli.md)。

## 运行产品检查

从仓库根目录运行以下命令。可移植转换栈需要 Rust 工具链和支持 HEVC 的 libheif。Apple 原生检查还需要符合要求的 macOS 和 Xcode 环境。

```bash
cargo metadata --locked --no-deps --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- --deny warnings
cargo test --locked --workspace --all-targets
cargo run --locked -p xdremux-cli -- --help
```

CLI 将任务交给运行时。运行时根据引擎生成的计划，调用输入、媒体、容器和编解码 crate。可移植实现与 Apple 适配器提供所需能力。

用户意图由 CLI、运行时和引擎处理。只有编码、源格式代际、路由、相机尾数据或容器选择对应不同的用户结果时，才将其公开为选项。

## 按组件职责修改

| Crate | 职责 |
| --- | --- |
| `xdremux-cli` | 公开命令及其用户可见行为。 |
| `xdremux-runtime` | 文件系统操作、输出发布、批处理执行、恢复和平台能力协调。 |
| `xdremux-engine` | 用户意图、源信息、确定性策略、能力要求和转换计划。 |
| `xdremux-source` 与 `xdremux-classification` | 输入检查、资产身份和分类。 |
| `xdremux-motion-photo` | 产品使用的 LibLivePhoto 接口、图像几何处理和文件对发布。 |
| `xdremux-hdr` 与 `xdremux-metadata` | HDR 和 Gain Map 计算及元数据操作。 |
| `xdremux-container`、`xdremux-heif`、`xdremux-codec` 与 `xdremux-format` | 产品的容器和编解码操作。共享二进制类型来自 `liblivephoto-format`。 |

下层 crate 提供某项格式操作，并不意味着产品公开了相应模式。引擎和运行时决定何时使用这些操作。独立库的边界见 [LibLivePhoto 接入契约](liblivephoto-integration.md)，产品执行流程见[架构文档](architecture.md)。

## 通过适配器使用 Apple 框架

Swift package 对外提供一个可执行产品 `xdremux-apple-adapter`，其 Swift 源码位于 `Sources/XDRemuxAppleAdapter/`。包内还包含 `XDCinematicNative` 和 `XDTextureNative` 两个目标，用于执行 Apple 原生操作。原有 Swift 转换、核心和验证器产品已删除。

保持以下职责边界：

- Rust 负责请求、结果、策略、路由、命名、分类、HDR、Motion Photo、批处理、验证和失败后的替代方案。
- 适配器调用 Apple 框架，返回 Rust 契约规定的观察结果或操作结果。
- 不得在 Swift 或原生目标中增加跨平台产品策略。
- 框架事实足够时，直接返回这些事实。输入是否可转换、是否具备有效 Portrait 资源，由 Rust 判断。

适配器是随产品分发的平台组件，不是另一套公开 CLI。运行时通过有版本号的 JSON 协议与适配器通信。每个辅助进程都有运行时限。机器可读结果写入 stdout，诊断信息写入 stderr。

`xdremux-runtime` 负责通信机制。引擎不启动辅助进程，也不实现通信层。macOS App 同样调用 Rust CLI 执行产品操作。

适配器的 ImageIO 探针报告 Gain Map、视差、Portrait Effects Matte 和语义蒙版等辅助资源。Rust 判断这些观察是否满足 Portrait 编辑契约。适配器构建检查平台实现；产品行为由 Rust 测试和 Rust 发起的消费者检查验证。

### 为新能力选择通信方式

对于启用沙盒的 App，如果能力进程需要独立沙盒、权限声明、生命周期管理或崩溃隔离，优先考虑 XPC。这是未来修改通信方式时的设计指导；当前适配器使用 JSON 进程协议。

更换通信方式不得改变引擎或公开 CLI 的行为。只有能力确实能从进程内互操作获益时，才使用进程内 C ABI。不要仅为移除辅助进程而默认采用 FFI。

每次辅助进程调用必须有运行时限，并显式回收子进程。诊断信息与机器可读结果必须分开。运行时必须检查 Apple 私有 API 的兼容性。私有 Objective-C 方法的运行时签名不支持某个 ABI 时，不得按该 ABI 调用。

## 构建与验证 macOS 包

完整 macOS 产品面向 Apple silicon（`arm64`）和 macOS 27 或更新版本。App、Swift package 和产品归档使用 macOS 27.0 部署目标。适配器在报告能力前拒绝不支持的主机。

[`rust-cli-core.yml`](../.github/workflows/rust-cli-core.yml) 工作流使用 `xcode-27` 运行器构建一个 arm64 产品包。打包实现位于 [`scripts/distribution/`](../scripts/distribution/)，保留根 Swift package 和两个原生目标。Linux、Windows 和 macOS 分别运行 Rust 核心检查。这些检查不能证明各平台均具备完整的 Apple 功能。

产品包包含：

- Rust CLI 和 Apple 适配器。
- 以校验和固定的原生依赖、依赖源码归档和许可文本。
- 使用相对加载路径并带有临时签名的必需动态库。

上传前，CI 在检出目录之外解包。它使用 `env -i` 和仅含系统目录的 `PATH` 启动包内程序。检查会转换真实 Samsung HEIF Motion Photo 和 OPPO ProXDR 样本，验证 Live Photo 文件对及 HDR 输出，确认输入未被修改，并拒绝再次写入已有文件对。

dyld 日志必须显示使用包内 libheif，不得显示从包外加载的非系统库。可移植打包检查命令为：

```bash
python3 -m unittest tests.test_macos_distribution
```

产品包检查只证明被测 arm64/macOS 27 主机上的可移植转换行为。Photographic Styles、Texture Styles 和 Cinematic 仍需要各自的框架、运行时、硬件和原生验收证据。

仓库不构建 Intel 产品包。个别 API 早于 macOS 27 引入，并不降低完整产品或私有 SPI 的支持要求。归档对固定暂存字节统一时间戳和所有者信息，但不据此声称编译器级构建可复现。产品包是带临时签名的 CI 产物，不是经过公证的发行版。

## 安装 Python 研究工具

Python 包要求 Python 3.11 或更新版本。它不再安装 CLI，不参与产品运行时，也不定义产品行为。

仅在研究流程需要 Python 时安装：

```bash
python -m pip install -e .
python -m unittest tests.test_apple_reverse_key1_training
```

包依赖 `Pillow` 和 `numpy`。可选 `training` 依赖会加入 PyTorch。训练和评估代码属于研究工具，可以保留在 Python 中。

## 查找对应文件

| 路径 | 用途 |
| --- | --- |
| `crates/` | Rust 产品实现与 Rust 测试。 |
| `Sources/XDRemuxAppleAdapter/` | 运行时调用的 Swift Apple 框架适配器。 |
| `Sources/XDCinematicNative/` 与 `Sources/XDTextureNative/` | 适配器使用的原生操作。 |
| `xdremux_py/` | Python 研究和训练工具。 |
| `apps/macos/XDRemuxApp/` | SwiftUI App、Xcode 工程、App 测试与构建脚本。 |
| `tests/` | Python 回归、仓库规则检查、验证框架自测和小型合成样本。 |
| `fixtures/` | 严格验证检查使用的版本化真实媒体。 |
| `scripts/` | 用于 CI、验证、Apple 检查、诊断、分发和性能测量的[仓库自动化](../scripts/README.md)。 |
| `docs/` | 当前指导和验证记录。历史审计位于 `docs/history/`。 |
| `research/` | OPPO Styles、Texture Styles、视频 Styles 和 Palette 研究。 |
| `benchmarks/` | 版本化性能基线；生成的测量结果另行保存。 |
| `research/oppo_styles/models/` | 可选研究模型及其文档。 |

## 构建与检查 macOS App

App 调用 Rust CLI。Swift 负责界面状态、队列，以及将 CLI 回执转换为 App 状态。App 将 CLI 和 Apple 适配器作为辅助程序打包。转换策略仍由 Rust 负责。

运行 `clean` 前，检查是否设置了 `XDREMUX_DERIVED_DATA`。脚本会删除该变量指定的目录；默认目录为 `.build/xcode`。

从仓库根目录运行 App 脚本，每次选择一项操作：

| 命令 | 结果 |
| --- | --- |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh run` | 构建 Release 并打开 App。 |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh build` | 构建 Release，不打开 App。 |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh debug` | 构建 Debug 并打开 App。 |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh verify` | 构建 App 并运行脚本中列出的检查。 |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh logs` | 输出已保存构建日志的最后 200 行。 |
| `apps/macos/XDRemuxApp/scripts/build_and_run.sh clean` | 删除配置的派生数据目录。 |

## 区分研究控制与公开接口

部分环境变量用于编码诊断、临时文件保留、风格渲染和研究模型选择。

只有产品行为确实需要时，才将研究变量加入普通用户命令。只有 Rust 产品实现和测试依赖某个研究开关时，才将其视为稳定的公开接口。

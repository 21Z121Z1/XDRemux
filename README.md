# XDRemux

[English](README.en.md) | 简体中文

XDRemux 将支持的厂商 ProXDR 照片转换为 ISO/TS 21496-1 HDR HEIC，也将支持的 Android Motion Photo 转换为 Apple Live Photo 文件对。

Rust `xdremux` CLI 自动识别源类型、HDR 结构和转换方式。macOS App 使用同一套产品实现。能否转换取决于文件的实际结构和资源，不能只按设备型号判断。

## 支持的任务

| 任务 | 结果或命令 |
| --- | --- |
| 转换 ProXDR 静态照片 | 含 ISO 增益图的 HDR HEIC。 |
| 转换支持的 Motion Photo | 标识和时间匹配的 HEIC 与 MOV 文件对。 |
| 封装 Live Photo | `--live-photo-output pvt` 创建包含已验证文件对和元数据的目录包。 |
| 请求 OPPO 相册兼容输出 | `--oppo-compatible` 适用于 ProXDR 静态照片。 |
| 添加 Apple 编辑资源 | 人像、摄影风格和纹理风格需要文档规定的源数据及 macOS 能力。 |
| 添加待处理的电影效果元数据 | 实验性选项 `--apple-cinematic` 为普通视频准备元数据，后续仍需原生处理。 |
| 批量转换 | `batch` 规划输出并逐项报告结果。 |
| 整理资产 | `categorize` 或 `batch --categorize` 按资产类型和拍摄模式分类。 |
| 检查文件 | `inspect` 报告源数据；`validate` 检查支持的输出。 |

完整 Mac 产品面向 Apple silicon（`arm64`）和 macOS 27。Rust 核心另有 Linux、Windows、macOS 构建及可移植测试。可移植构建通过，不代表 Apple 框架或 Photos 编辑功能可用。

选择编辑功能前，先查看[支持设备与源文件证据](docs/supported-devices.md)和 [Apple 功能要求](docs/apple-features.md)。

## 构建 Rust CLI

可移植转换需要当前 Rust 工具链及支持 HEVC 的 libheif。完整 Mac 包还需要 Apple 适配器，构建时使用 Xcode 27，部署目标为 macOS 27。

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
cargo build --locked --release -p xdremux-cli
./target/release/xdremux --help
```

下列示例假设 `xdremux` 已加入 `PATH`。从源码构建时，将命令中的 `xdremux` 替换为 `./target/release/xdremux`。

开发时也可以通过 Cargo 运行：

```bash
cargo run --locked -p xdremux-cli -- --help
```

App、适配器和可迁移安装目录的打包方法见[开发文档](docs/development.md)。

## 转换单张照片

> [!CAUTION]
> 转换 ProXDR 静态照片时，省略 `--output` 会在转换成功后替换输入文件。需要保留原图时，指定独立输出路径。

转换支持的 ProXDR 静态照片：

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

转换支持的 Motion Photo：

```bash
xdremux convert --input IMG_001.jpg
```

Motion Photo 转换生成配套的 HEIC 和 MOV，源文件保持不变。未指定 `--output` 时，程序选择不会与已有文件对成员冲突的名称。

ProXDR 输出需要面向 OPPO 相册时，添加兼容选项：

```bash
xdremux convert --input IMG_001.heic --output IMG_001_oppo.heic --oppo-compatible
```

Motion Photo 转换拒绝 `--oppo-compatible`。PVT 封装和 Apple 编辑选项各有前置条件，见 [CLI 参考](docs/cli.md)。

## 批量转换

```bash
xdremux batch --input-dir photo_dump/ --recursive --output-dir converted/
```

批处理先规划输出路径，再写入文件。`--jobs` 控制并发数，每个输入单独报告失败。使用 `--json` 获取结构化执行回执。

Live Photo 检查点复用要求源文件和输出身份匹配，不会跳过任意已有输出。具体条件见[批处理参数与恢复](docs/cli.md#batch)。

## 按类型整理资产

```bash
xdremux categorize --input photo_dump/ --output-dir categorized/ --dry-run
xdremux categorize --input photo_dump/ --output-dir categorized/
```

第一条命令只报告计划，不发布文件。第二条命令将资产复制到分类目录。通过验证的 Live Photo 照片和 MOV 作为同一资产处理。使用 `batch --categorize` 可直接把转换结果放入分类目录。

## 检查与验证

```bash
xdremux inspect IMG_001.heic --json
xdremux validate IMG_001_hdr.heic --json
```

`inspect` 报告从源文件解析出的信息。普通 `validate` 检查支持的 HDR HEIF、Live Photo 文件对或 PVT 目录包。结构验证不能证明相册显示或交互编辑行为。

## Apple 编辑功能

Rust 负责选择功能、准备资源、验证结果和发布输出。Swift 适配器只执行需要 Apple 框架的操作。

`--apple-portrait` 请求完整人像资源集。`--apple-portrait-oppo` 使用 OPPO 深度和已有有效蒙版，不调用 Vision 分割。后者的人像效果有所减少，仍需 macOS 上公开的 ImageIO 和 Core Image API。

`--apple-styles` 在受支持的 macOS 环境生成摄影风格和推理得到的纹理风格。`--apple-cinematic` 为普通视频生成待处理元数据。电影效果达到待处理状态，不保证 Photos 能处理所有输入。

源数据要求和验证边界见 [Apple 功能](docs/apple-features.md)与[电影效果约定](docs/cinematic.md)。

## 代码与文档

CLI 调用 Rust runtime。runtime 通过媒体 crate 和平台适配器执行 engine 生成的计划。产品策略保留在 Rust 中，Python 工具用于研究和模型训练。

| 文档 | 用途 |
| --- | --- |
| [CLI 参考](docs/cli.md) | 选项、默认值、输出安全和退出码。 |
| [架构说明](docs/architecture.md) | 模块职责和媒体约定。 |
| [开发文档](docs/development.md) | 构建、打包和贡献检查。 |
| [测试政策](docs/quality/testing.md) | 各类修改所需的证据。 |
| [文档索引](docs/README.md) | 研究工具、验证记录和其他指南。 |

`fixtures/` 中纳入版本控制的媒体用于真实文件测试。样本身份和来源记录属于测试约定的一部分。

## 许可证

XDRemux 的原创程序代码采用 [GPL-3.0-only](LICENSE) 许可，版权人同时对 v1.0–v1.4 的自有代码追加该授权。真实照片及视频样张、Core ML 模型权重**不属于 GPLv3**；具体范围和历史版本说明见[授权说明](LICENSING.zh-CN.md)。

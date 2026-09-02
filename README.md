# XDRemux

[English](README.en.md) | 简体中文

XDRemux 将支持的厂商 HDR 照片转换为 ISO/TS 21496-1 HDR HEIC，并把支持的 Android Motion Photo 转换为 Apple Live Photo。

产品只有一个入口：Rust `xdremux` CLI。输入类型、源代际、HDR / Gain Map 结构以及 Motion Photo 路由均由程序自动识别。

## 能做什么

| 输入 / 意图 | 结果 |
| --- | --- |
| ProXDR 照片 | ISO/TS 21496-1 HDR HEIC |
| 支持的 Motion Photo | Apple Live Photo HEIC + MOV |
| `--oppo-compatible` | 面向 OPPO 相册的 ProXDR 输出 |
| `categorize` / `batch --categorize` | 按资产类型和拍摄模式分类 |
| `inspect` / `validate` | 检查源文件和独立验证输出 |

正常转换不要求用户选择设备代际、codec、Gain Map layout、camera tail 或 routing。这些都是程序根据输入事实和期望结果自动决定的实现细节。

## 构建

跨平台转换栈需要当前 Rust toolchain，以及带 HEVC 支持的 libheif。

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
cargo build --release -p xdremux-cli
./target/release/xdremux --help
```

开发时也可以直接通过 Cargo 运行：

```bash
cargo run -p xdremux-cli -- --help
```

## 转换

标准 ProXDR 转换：

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_hdr.heic
```

支持的 Motion Photo 使用同一个命令，程序会自动识别：

```bash
xdremux convert --input IMG_001.jpg
```

Motion Photo 会生成同 basename 的 HEIC + MOV Live Photo 资源对，源 Motion Photo 不会被修改。

需要面向 OPPO 相册的 ProXDR 输出时：

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic \
  --oppo-compatible
```

`--oppo-compatible` 只适用于 ProXDR 静态照片，不适用于 Motion Photo 转换。

> [!IMPORTANT]
> 对 ProXDR 静态照片，如果省略 `--output`，目标路径就是输入路径，并通过原子 publication 替换。源文件重要时请保留原始副本。

## 批处理

```bash
xdremux batch \
  --input-dir photo_dump/ \
  --recursive \
  --output-dir converted/
```

Batch 支持重复的文件/目录输入、有界 `--jobs`、确定性输出规划、逐项失败隔离、checkpoint/resume、基于 provenance 的安全复用、结构化 JSON receipt，以及可选的 `--categorize` publication。

完整命令契约见 [`docs/cli.md`](docs/cli.md)。

## 分类

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

通过验证的 Live Photo HEIC 和 MOV 会作为一个资产处理。使用 `--dry-run` 可以只查看规划而不写入文件。

## 检查与验证

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json

xdremux validate output.heic
xdremux validate output.heic --json
```

`inspect` 报告从源文件解析出的事实；`validate` 会自动验证 ISO HDR HEIF 或 Live Photo 资源对。

## Apple 编辑能力

摄影风格和 Apple 人像目前仍处于迁移边界内。目标架构由 Rust 持有产品 policy、orchestration、数据模型和 CLI；一个很薄的 Apple-native adapter 只负责调用 Core Image、Vision、Core ML、AVFoundation 等平台 framework。

当前支持和验收边界见 [`docs/apple-features.md`](docs/apple-features.md)。

## 架构

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

新的产品行为进入这套 Rust 产品栈。迁移期实现和研究工具不定义公开 CLI 契约。

## 文档

- [CLI 参考](docs/cli.md) — 命令、默认行为、退出状态和 batch 可靠性
- [Apple 功能](docs/apple-features.md) — 摄影风格和人像的迁移边界
- [支持设备](docs/supported-devices.md) — 输入兼容性证据
- [开发文档](docs/development.md) — 架构、所有权和构建/测试流程
- [测试政策](docs/quality/testing.md) — 必需验证证据
- [文档索引](docs/README.md) — 其他技术文档

`fixtures/` 下版本化保存的真实媒体 corpus 用于 ProXDR 和 Motion Photo 的严格真实文件 gate。

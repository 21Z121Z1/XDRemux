# XDRemux

[English](README.en.md) | 简体中文

XDRemux 将 OPPO、OnePlus 和 realme 设备拍摄的 ProXDR 照片转换为更容易在其他系统中识别的 HDR HEIC。

它可以生成标准 ISO HDR 文件，也可以按需要保留 OPPO 相册兼容性，或为 Apple Photos 生成摄影风格和人像编辑数据。

## 功能

| 模式 | 用途 |
| --- | --- |
| 标准 HDR | 在支持 ISO HDR Gain Map 的系统中显示 HDR |
| OPPO 兼容 | 保持在 OPPO 相册中的 HDR 显示兼容性 |
| Apple 摄影风格 | 在 Apple Photos 中使用摄影风格编辑 |
| Apple 人像 | 在 Apple Photos 中继续调整景深和焦点 |

标准 HDR 是默认模式。Apple 摄影风格和 Apple 人像可以组合使用；OPPO 兼容模式不能与 Apple 模式同时启用。

## 系统要求

- macOS 15 或更高版本。
- Swift 6 工具链。
- Apple 人像转换需要 `zstd`；JPEG 人像桥接还需要 `ultrahdr_app` 位于 `PATH`。

从源码使用 Apple 功能前，请先运行一次完整的 `swift build`。

## 快速开始

```bash
git clone https://github.com/21Z121Z1/XDRemux.git
cd XDRemux
swift build
```

转换单张照片：

```bash
swift run xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic
```

批量转换目录：

```bash
swift run xdremux batch \
  --input-dir photo_dump/ \
  --output-dir converted/
```

> [!IMPORTANT]
> 省略 `--output` 或 `--output-dir` 时会覆写输入文件。转换前请备份原片。

## 常用模式

标准 HDR：

```bash
swift run xdremux convert --input IMG_001.heic --output IMG_001_iso.heic
```

OPPO 相册兼容：

```bash
swift run xdremux convert --oppo-compatible --input IMG_001.heic --output IMG_001_oppo.heic
```

Apple 摄影风格：

```bash
swift run xdremux convert --apple-photographic-styles --input IMG_001.heic --output IMG_001_styles.heic
```

Apple 人像：

```bash
swift run xdremux convert --apple-portrait --input IMG_001.heic --output IMG_001_portrait.heic
```

查看所有公开参数：

```bash
swift run xdremux --help
```

## 支持范围

XDRemux 面向能够拍摄 ProXDR HEIC 的 OPPO、OnePlus 和 realme 设备。不同机型可能使用不同的 Gain Map 编码和厂商元数据；已知设备列表见[支持设备](docs/supported-devices.md)。

Apple 人像要求输入照片包含可恢复的景深数据。普通照片不会仅因为启用该选项就自动变成人像照片。

## 已知限制

- Apple 摄影风格和 Apple 人像仍属实验功能，结果可能随设备和 macOS/iOS 版本变化。
- Apple 摄影风格公开版固定使用 constrained-solver；当前 final-HEIC 场景输入仍是研究候选，输出 manifest 保持 `productionEligible=false`。
- 转换后的照片在 OPPO 相册中再次编辑并保存后，HDR Gain Map 或 HDR 元数据可能丢失。
- 离线容器验证不能替代 Apple Photos 或 OPPO 相册的实机显示与保存重开验证。
- 项目目前以源码方式发布，尚未提供签名的通用安装包。

## 文档

- [CLI 完整参考](docs/cli.md)
- [Apple 摄影风格与人像](docs/apple-features.md)
- [开发、构建与 Swift Package 集成](docs/development.md)
- [支持设备](docs/supported-devices.md)
- [技术实现与验证资料](docs/xdremux/README.md)
- [English documentation](docs/README.en.md)

## Disclaimer

本工具仅供技术研究使用。转换前请备份原始文件。作者不承担因使用本工具造成数据丢失的责任。

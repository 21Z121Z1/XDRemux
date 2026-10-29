# XDRemux CLI for macOS

## English

### Requirements

Run this package on Apple silicon (`arm64`) with macOS 27. Keep the complete directory together. It does not require a repository checkout, Rust, SwiftPM, an Xcode project, or a Homebrew runtime installation.

Use an artifact from a successful complete workflow run. An unfinished build does not establish acceptance. `manifest.json` records the product revision, architecture, and macOS 27.0 deployment target. CI records the host OS used to verify the relocated package.

### Run a conversion

Specify a separate output path to retain the ProXDR source. Omitting `--output` for a ProXDR still replaces that source after conversion succeeds.

```bash
./bin/xdremux --lang en --help
./bin/xdremux convert --input "/path/to/IMG_001.heic" --output "/path/to/IMG_001_hdr.heic"
./bin/xdremux validate "/path/to/IMG_001_hdr.heic"
```

`bin/xdremux` finds `libexec/xdremux-apple-adapter` automatically. Non-system runtime libraries are in `lib/`. libheif includes its HEVC codecs. Runtime plugin discovery and environment-variable setup are not required.

The package smoke test covers Motion Photo and standard HDR conversion, pair validation, and unchanged inputs. Apple editing features still require their documented SDK, OS, frameworks, and hardware. Styles, Texture, and Cinematic require macOS 27 and separate native evidence.

A successful build or package test does not establish private API availability, native processing, interactive Photos editing, or app acceptance.

### Select a language

The selection order is `--lang`, `XDREMUX_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG`, then `en`. Empty environment values are ignored. Supported `zh` locales, including `zh_CN.UTF-8`, select Simplified Chinese. `C` and `POSIX` select English.

Unsupported environment locales fall back to English. An unsupported explicit `--lang` value is a usage error. JSON, subcommands, option names, enum values, schemas, and exit codes remain unchanged. Clap parser details and lower-level library diagnostics may remain English.

### Signatures and notices

The binaries have ad-hoc signatures. They do not have Developer ID signatures or notarization. Gatekeeper, download quarantine, and interactive Photos editing require separate acceptance checks. Do not disable system-wide security protections.

`share/licenses/xdremux/` contains the complete **GPL-3.0-only** text and the project's licensing scope. The original photographs/videos in `fixtures/` and Core ML model packages are excluded from that grant. `share/licenses/rust/` preserves resolved Rust dependencies' original notices and `share/licenses/rust-dependencies.json` inventories their versions, source identities, and notice checksums. Native dependency notices are in `share/licenses/`; their pinned source archives are in `share/sources/`. x265 is GPL-2.0-or-later; libheif and libde265 are LGPL-3.0-or-later; libwebp is BSD-3-Clause.

`CORRESPONDING_SOURCE.md` links to the **exact XDRemux source revision** recorded in `manifest.json` and the relevant build instructions. Retain these files when redistributing. A passing package test does not itself establish compliance with every applicable license, patent, SDK, or source-offer obligation.

## 简体中文

### 运行条件

在 Apple silicon（`arm64`）和 macOS 27 上运行此包，保留完整目录结构。运行不需要克隆仓库，也不需要 Rust、SwiftPM、Xcode 项目或 Homebrew 运行时安装。

使用整个工作流成功完成后的产物。构建尚未完成，不能视为验收通过。`manifest.json` 记录产品版本对应的提交、架构和 macOS 27.0 部署目标。CI 记录验证目录迁移后运行所用的主机系统。

### 执行转换

需要保留 ProXDR 源文件时，指定独立输出路径。ProXDR 静态照片省略 `--output` 后，会在转换成功时替换源文件。

```bash
./bin/xdremux --lang zh-CN --help
./bin/xdremux convert --input "/path/to/IMG_001.heic" --output "/path/to/IMG_001_hdr.heic"
./bin/xdremux validate "/path/to/IMG_001_hdr.heic"
```

`bin/xdremux` 自动查找 `libexec/xdremux-apple-adapter`。非系统运行库位于 `lib/`，libheif 已包含 HEVC 编解码器，无需配置运行时插件发现或环境变量。

包冒烟测试覆盖 Motion Photo、标准 HDR 转换、文件对验证及输入不变检查。Apple 编辑功能仍需文档规定的 SDK、系统、框架和硬件。摄影风格、纹理风格及电影效果需要 macOS 27 和独立原生证据。

构建或包测试通过，不证明私有 API 可用、原生处理成功、Photos 交互编辑或 App 验收通过。

### 选择语言

选择顺序为 `--lang`、`XDREMUX_LANG`、`LC_ALL`、`LC_MESSAGES`、`LANG`，最后为 `en`。空环境变量会被忽略。`zh_CN.UTF-8` 等受支持的 `zh` 区域设置选择简体中文，`C` 和 `POSIX` 选择英文。

不支持的环境语言回退到英文。显式指定不支持的 `--lang` 值时返回用法错误。JSON、子命令、选项名、枚举值、模式和退出码保持不变。Clap 参数诊断详情及底层库诊断仍可能使用英文。

### 签名与许可声明

二进制仅有 ad-hoc 签名，没有 Developer ID 签名，也未公证。Gatekeeper、下载隔离和 Photos 交互编辑需要独立验收。不要关闭全系统安全保护。

`share/licenses/xdremux/` 包含项目完整的 **GPL-3.0-only** 许可证和适用范围说明。`fixtures/` 中的照片、视频及 Core ML 模型包不在 GPL 授权范围内。`share/licenses/rust/` 收录 Rust 依赖的原始许可文本，`share/licenses/rust-dependencies.json` 记录依赖版本、来源与许可文件校验和。`share/licenses/` 还保留原生依赖声明，`share/sources/` 提供固定校验和的原生依赖源码包。x265 使用 GPL-2.0-or-later，libheif 与 libde265 使用 LGPL-3.0-or-later，libwebp 使用 BSD-3-Clause。

`CORRESPONDING_SOURCE.md` 提供与 `manifest.json` 所记载提交**精确对应**的项目源码地址及构建说明。再分发时保留这些文件。包测试通过本身不等于所有许可、专利、SDK 及源码提供义务均已满足。

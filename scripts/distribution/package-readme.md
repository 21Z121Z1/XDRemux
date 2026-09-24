# XDRemux — canonical Rust CLI for macOS

## English

Keep this entire directory together. No checkout, Rust, SwiftPM, Xcode project,
or Homebrew runtime installation is required. The validation target is macOS 26
for the architecture recorded in `manifest.json`; this is not a Universal 2 binary.
Use artifacts from a successful complete workflow run, not an unfinished build.

```bash
./bin/xdremux --lang en --help
./bin/xdremux convert --input "/path/to/IMG_001.heic" --output "/path/to/IMG_001_styles.heic" --apple-styles
./bin/xdremux validate "/path/to/IMG_001_styles.heic"
```

`bin/xdremux` discovers `libexec/xdremux-apple-adapter` automatically. All non-system
runtime dylibs are in `lib/`. libheif's HEVC codecs are built in; no runtime plugin
search or environment-variable configuration is needed.

Language precedence: `--lang`, `XDREMUX_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG`, `en`.
Empty environment values are ignored. `zh_CN.UTF-8` and other `zh` locales select
Simplified Chinese; `C` and `POSIX` select English. Unsupported environment locales
fall back to English; an unsupported explicit `--lang` is a usage error.
JSON, subcommands, option names, enum values, schemas, and exit codes are invariant.
Clap's detailed parser errors and lower-level library diagnostics may remain English.

Binaries have ad-hoc signatures, not Developer ID signatures or notarization.
Gatekeeper/download quarantine and interactive Photos editability are separate
end-user acceptance checks; do not disable system-wide security protections.

`share/licenses/` contains upstream license notices; `share/sources/` contains the
checksum-pinned native source archives. x265 is GPL-2.0-or-later; libheif/libde265
are LGPL-3.0-or-later. These notices do not relicense XDRemux. A production publisher
must resolve the project's distribution license and corresponding-source duties.
The exact product revision is recorded in `manifest.json`.

## 简体中文

请保留完整目录结构。运行不需要克隆仓库、Rust、SwiftPM、Xcode 项目或
Homebrew 运行时。验证目标为 macOS 26 的对应架构，不是 Universal 2。
请使用整个 workflow 已成功完成的产物，不要将未完成的构建视作验收通过。

```bash
./bin/xdremux --lang zh-CN --help
./bin/xdremux convert --input "/path/to/IMG_001.heic" --output "/path/to/IMG_001_styles.heic" --apple-styles
./bin/xdremux validate "/path/to/IMG_001_styles.heic"
```

CLI 自动找到 `libexec/xdremux-apple-adapter`，所需非系统 dylib 位于 `lib/`。
HEVC codec 已内置到 libheif，无需手动配置插件或环境变量。
语言优先级为 `--lang` → `XDREMUX_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG` → `en`。
忽略空环境变量；`zh_CN.UTF-8` 等 `zh` locale 使用简体中文；`C`、`POSIX` 使用英文。
不支持的环境 locale 回退英文，显式指定不支持的 `--lang` 则返回参数错误。
JSON、命令、选项、枚举、schema 和退出码保持不变。Clap 参数错误详情及底层库诊断
可能仍使用英文。

产物仅使用 ad-hoc 签名，不含 Developer ID 签名或 notarization。下载后的 Gatekeeper
行为和 Photos 交互编辑能力需独立验收；不要关闭全系统安全保护。
`share/licenses/` 和 `share/sources/` 包含原生依赖许可及固定校验和的源代码。
x265 使用 GPL-2.0-or-later，libheif/libde265 使用 LGPL-3.0-or-later；这些声明不改变
XDRemux 自身许可。生产发布者仍需确认产品许可及 corresponding-source 义务。

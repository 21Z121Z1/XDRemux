# Apple 摄影风格与人像

[English](apple-features.en.md) | 简体中文

在标准 HDR 输出之外，XDRemux 还能生成让照片在 Apple Photos 里可编辑的数据。两个功能默认关闭，都还是实验性的。

| 功能 | 你会得到什么 |
| --- | --- |
| Apple 摄影风格 | 在 Apple Photos 里切换摄影风格，调整色调、色彩和强度 |
| Apple 人像 | 在 Apple Photos 里调整虚化程度，条件允许时还能重新对焦 |
| 两个一起开 | 一个文件里同时保住 HDR、摄影风格和人像编辑 |

所有数据都从你这张照片本身算出来，不会从别的照片复制画面或编辑参数。

## 使用要求

- macOS 15 或更高版本
- 从源码运行时先跑一次完整的 `swift build`
- Apple 人像需要 `zstd`（`brew install zstd`）
- JPEG 人像还需要 `ultrahdr_app`
- 当前系统要能提供 Apple 的图像分析能力

系统能力缺失时转换会直接报错，不会写一个空壳资源糊弄过去。

## Apple 摄影风格

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux 会分析照片的画面、亮度、色彩和人物/天空等区域，生成摄影风格需要的数据。只有真正检测到的区域会写进去。

生成的风格参数会跟原生 iPhone 照片的编辑响应做对照：如果这张照片在编辑器里的表现超出了原生样本的范围，求解会把这一项也纳入目标一起修正，并保证结果不比修正前差。本来就正常的照片走快路径，只在最后验证一次。

这个模式保留标准 HDR 输出，不能和 `--oppo-compatible` 同时用。

求解很吃 CPU，批量处理请用 release 构建：`swift build -c release` 之后跑 `.build/release/xdremux`。

## Apple 人像

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

源照片必须是用人像模式拍的，并且景深、对焦和未虚化的原图都还留在文件里。XDRemux 把这些转成 Apple Photos 能继续编辑的形式，尽量保住原来的对焦位置和虚化程度。

普通非人像照片没有这些数据，人像转换就不可用。只开人像的批量任务会把它记为失败；如果同时开了摄影风格，这张照片会降级成只输出摄影风格。

每个成功的人像输出旁边还会有一个 `<输出文件名>.portrait-manifest.json`，记录输入带了什么、转换选了什么、有哪些警告。这个文件不需要跟照片一起导进 Apple Photos。

## JPEG 人像

有些 OPPO 人像照片外层是 JPEG。这类文件只在开启 `--apple-portrait` 时接受，输出仍然是 HEIC。批量处理要显式指定：

```bash
swift run xdremux batch \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

其他模式（标准 HDR、只开摄影风格、OPPO 兼容）仍然只接受 HEIC。

## 两个一起开

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

只产出一个文件。`--oppo-compatible` 面向 OPPO 相册，Apple 模式面向 Apple Photos，两者不能组合 —— CLI 会在写文件之前就拒绝这个参数组合。

## 验证到了什么程度

已经做到的检查：

- 输出文件能重新打开
- HDR 数据和 Apple 附加资源的引用关系能正确解析
- 摄影风格和人像资源能通过仓库自带的检查工具
- App 和 CLI 对同一个转换请求产出相同结果

**没有**做到的：把文件导进真机 Apple Photos，编辑、保存、退出、重新打开，确认编辑能力还在。这一轮目前只能手动验，不构成公开的通过项。

所以这两个功能的输出清单里始终标着"未达到生产可用"。离线的结构检查也不等于在 iPhone、Mac 或 OPPO 相册里的真实显示效果，更不代表所有机型、焦段和系统版本都验过。

## 研究开关

有若干 `XDREMUX_RESEARCH_*` 和 `XDREMUX_STYLES_*` 环境变量可以切换实验性的求解路径。它们**默认都不需要设置**，一旦启用，输出清单会标记为研究模式并排除在生产判定之外。具体变量见[开发文档](development.md)。

实现细节和验收资料见[技术实现索引](xdremux/README.md)、[ISO 容器审计](xdremux/iso-conformance-audit-20260511.md)和[验证说明](validation/README.md)。

# Apple 摄影风格与人像

[English](apple-features.en.md) | 简体中文

XDRemux 可以在标准 HDR 输出之外，为 Apple Photos 生成摄影风格或人像编辑资源。这两项功能默认关闭，且仍处于实验阶段。

## 功能范围

| 功能 | 用户结果 |
| --- | --- |
| Apple 摄影风格 | 在 Apple Photos 中切换摄影风格，并调整色调、色彩和强度 |
| Apple 人像 | 在 Apple Photos 中调整模拟光圈，并在可用时重新选择焦点 |
| 组合模式 | 在一个最终 HEIC 中同时保留 HDR、摄影风格和人像编辑能力 |

这些功能只处理当前输入照片，不会从其他照片复制画面或编辑资源。

## 使用要求

- macOS 15 或更高版本。
- 从源码运行时，先执行一次不限定 product 的 `swift build`。
- Apple 人像需要 `zstd` 位于 `PATH`。
- JPEG 人像桥接还需要 `ultrahdr_app` 位于 `PATH`。
- 当前系统必须提供所需的 Apple 图像分析能力。

缺少系统能力时，转换会返回明确错误，不会写入伪造的空资源。

## Apple 摄影风格

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux 根据当前照片生成 Apple Photos 所需的语义区域和摄影风格资源。只有实际检测到的有效区域会写入输出；面积较小但有效的区域仍会保留。

该模式继续保留标准 HDR 输出。它不能与 `--oppo-compatible` 同时启用。

## Apple 人像

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

输入必须包含可恢复的厂商景深、对焦和未虚化图像资源。XDRemux 会把这些数据转换成 Apple Photos 可以继续编辑的人像资源，并尽量保留原始对焦位置和模拟光圈。

普通非人像照片缺少必要景深资源时，人像转换不可用。仅启用人像的批量任务会记录失败；与摄影风格组合时，该照片可以降级为摄影风格输出并产生 warning。

每个成功的人像输出还会生成 `<output>.portrait-manifest.json`，记录输入能力、转换选择、warning 和可复查的验证信息。

## JPEG 人像桥接

Apple 人像也支持包含标准 HDR Gain Map 和完整厂商人像资源的 OPPO HDR JPEG。批量处理 JPEG 时需要显式选择：

```bash
swift run xdremux batch \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

JPEG 输入只在启用 `--apple-portrait` 时接受，可以同时启用摄影风格。标准 HDR、单独摄影风格和 OPPO 兼容模式仍使用 HEIC 输入。

## 组合与冲突

摄影风格和人像可以同时启用：

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

组合模式只生成一个最终 HEIC。`--oppo-compatible` 面向 OPPO 相册，Apple 模式面向 Apple Photos，二者不能组合，CLI 会在写文件前拒绝该参数组合。

## 当前验证状态

当前实现已覆盖以下离线检查：

- 输出容器可以重新打开。
- 标准 HDR Gain Map 和 Apple auxiliary 引用可以解析。
- 摄影风格和人像资源可以通过仓库 validator。
- App 与 CLI 对同一转换请求生成相同输出。
- 已有真实样本通过有限的 macOS Photos 编辑、重新对焦和保存重开检查。

这些结果不代表所有设备、焦段、系统版本或 Apple Photos 版本都已验收。离线结构验证也不等同于 iPhone、Mac 或 OPPO 相册中的真实显示效果。

实现与验收资料见[技术实现索引](xdremux/README.md)、[ISO 容器审计](xdremux/iso-conformance-audit-20260511.md)和[验证说明](validation/README.md)。阶段性样本日志、固件字段和逆向证据保留在研究资料中，不作为产品承诺。

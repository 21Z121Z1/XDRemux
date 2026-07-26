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

公开版本只提供一条 constrained-solver 摄影风格路径。它会把当前照片的 Base、RGB Gain、方向、GTC 和相关元数据组织成 per-photo SceneBundle，但 final-HEIC 缺少 capture-time pre-LTM 输入，因此输出 manifest 仍保持 `productionEligible=false`。

constrained-solver 还会测量照片在编辑器中的 Tone@Color100 响应（ROI 优先使用皮肤区域，无人像时退化为暖色候选区）。当同照片 identity 对照的 OKLab hue 或 R/G 响应超出原生样本包络时，响应约束会进入求解目标，且验收要求结果不劣于 identity；本就合规的照片走快路径，只在选出结果后验证一次，若检出回归会自动升级为完整响应目标重解。响应包络、判定结果和 ROI 信息都会写入 solver 输出目录的 `solver-result.json`（`responseObjective` 字段）。

研究性环境变量（默认都不需要设置）：

- `XDREMUX_STYLE_RESPONSE_OBJECTIVE=off` 恢复纯中性重建目标（v5 行为，结果与旧版逐位一致）。
- `XDREMUX_STYLES_LINEAR_THUMBNAIL_MODE=seam-min-ratio` 选择研究性 Linear Thumbnail seam 变体；该变体会标记 `researchOverrideActive` 并排除生产判定。

摄影风格求解为计算密集路径，批量转换建议 release 构建（`swift build -c release` 后运行 `.build/release/xdremux`）。

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
- 当前可复现证据限于离线容器、ImageIO、helper 和 App bundle 检查；没有把真实 Photos 保存重开结果作为公开产品通过项。

这些结果不代表所有设备、焦段、系统版本或 Apple Photos 版本都已验收。离线结构验证也不等同于 iPhone、Mac 或 OPPO 相册中的真实显示效果。

实现与验收资料见[技术实现索引](xdremux/README.md)、[ISO 容器审计](xdremux/iso-conformance-audit-20260511.md)和[验证说明](validation/README.md)。阶段性样本日志、固件字段和逆向证据保留在研究资料中，不作为产品承诺。

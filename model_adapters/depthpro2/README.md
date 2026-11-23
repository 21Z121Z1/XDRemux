# Depth Pro 2 模型适配器

[English](README.en.md) | 简体中文

这个可插拔的 Core ML 适配器用于接入 Apple Depth Pro 2（`apple/DepthPro2-coreml`），不在 XDRemux 源码仓库或分发压缩包中内嵌或随包分发苹果模型权重。用户在明确同意上游 [Apple Machine Learning Research License](https://huggingface.co/apple/DepthPro2-coreml/blob/4e607a682f993bbf95035054fc78ba9ffc81faf2/LICENSE) 后，外部模型包存放于用户模型目录（`~/Library/Application Support/XDRemux/Models`，可通过 `XDREMUX_MODEL_DIR` 覆盖）。

## 能力与职责边界

- **视角焦距估计（`depthpro2-fov_512x896.mlpackage`）**：在 `xdremux-apple-adapter` 内对固定视角视频采样抽帧运行推理，在输入视频缺少已记录的 `FocalLengthIn35mmFormat` 元数据时，为 `xdremux convert --apple-cinematic` 输出标准的 `CinematicCameraEstimate` JSON（`schema_version: 1`）。
- **静态图视差估计（`depthpro2-b_w4.mlpackage` / `depthpro2-s_w4.mlpackage`）**：在 `xdremux-apple-adapter` 内通过 Core ML 与 Metal 提供逆深度到视差的推理原语，使 `xdremux convert --apple-portrait` 可以处理没有内置 OPPO `rear.depth` 的 4:3 与 3:4 ProXDR 或 ISO Gain Map 照片。
- **产品策略归属 Rust**：Swift 适配器仅负责 Core ML 推理、方向校正与 ImageIO/Vision 原语；Rust 负责相机观测准入、等效焦距决策、REND 生成、辅助图封装、结构校验及原子发布。

## 可插拔安装

在确认接受 Apple AMLR 许可条款后，从仓库根目录执行：

```bash
python3 scripts/distribution/install_depthpro2.py \
  --accept-apple-amlr-license \
  --models s,b,fov \
  --bin-dir /path/to/xdremux/bin
```

`install_depthpro2.py` 会执行以下步骤：

1. 从 `apple/DepthPro2-coreml` 修订版本 `4e607a682f993bbf95035054fc78ba9ffc81faf2` 下载 `depthpro2-s_w4.mlpackage`、`depthpro2-b_w4.mlpackage` 和 `depthpro2-fov_512x896.mlpackage` 到 `--models-dir`，并逐一校验 `weight.bin` 的 SHA-256 摘要。
2. 在 `--models-dir` 中写入 `.apple-amlr-license-accepted.json`，供 Core ML 原语验证已明确授权。
3. 可选地在 `--bin-dir`（`xdremux` 与 `xdremux-apple-adapter` 所在目录）中发布自包含的 `xdremux-depthpro2` 启动器，且不覆盖已有同名文件。

## 模型权重 SHA-256 校验值（`4e607a682f993bbf95035054fc78ba9ffc81faf2`）

| 模型包 | 用途 | `weight.bin` SHA-256 |
| --- | --- | --- |
| `depthpro2-s_w4.mlpackage` | 快速 ANE/GPU 视差估计 | `24b741e5608cbdd99bc2b26d4925c410b2d415637c36b1c0b51050af9c9be1db` |
| `depthpro2-b_w4.mlpackage` | 高精度 CPU+GPU 视差估计 | `6f47f78fcc7054239bd1f9653b82179e622b0adff0c30877b495f62bf56c37bc` |
| `depthpro2-fov_512x896.mlpackage` | 视角与等效焦距估计 | `1d95a062127b46b7d81d393aac28d58b9dec03f5535409267ee32a14ccf984f5` |

## 与 `xdremux` 配合使用

```bash
# 当视频缺少 EXIF 等效焦距时，通过 Depth Pro 2 FOV 估计生成待处理 Cinematic 视频
XDREMUX_DEPTHPRO2=/path/to/xdremux/bin/xdremux-depthpro2 \
  xdremux convert --input fixed-view.mov --output pending.mov --apple-cinematic

# 或直接运行视角估计器生成 camera.json 后手动传入
python3 model_adapters/depthpro2/estimate.py --input fixed-view.mov --output camera.json
xdremux convert --input fixed-view.mov --output pending.mov \
  --apple-cinematic --cinematic-camera-estimate camera.json

# 将 4:3 / 3:4 静态照片通过 Depth Pro 2 + Vision/ImageIO 转换为 Apple 人像模式照片
xdremux convert --input photo_4x3.heic --output portrait.heic --apple-portrait
```

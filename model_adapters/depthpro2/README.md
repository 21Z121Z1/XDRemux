# Depth Pro 2 模型适配器

[English](README.en.md) | 简体中文

Depth Pro 2 适配器从本地加载 Apple 的 Core ML 或 PyTorch 模型权重。macOS 默认模型目录为 `~/Library/Application Support/XDRemux/Models`，Linux 独立 PyTorch 适配器默认使用 `~/.local/share/XDRemux/Models`。可以通过 `XDREMUX_MODEL_DIR` 更改目录。安装任一后端前请阅读许可条款。[Apple Machine Learning Research Model License](https://huggingface.co/apple/DepthPro2-S/blob/903a1c1ade918831d138adcc4aedbcc8f4da0078/LICENSE)

## 用 PyTorch 生成半尺寸或全尺寸深度图

S、B 模型支持任意宽高比的静态图像。命令输出 Float32 NumPy 数组（`.npy`），保存以米为单位的光轴深度。应用 EXIF 方向后，默认将宽高分别减半并向上取整。例如 `8192×6144` 会输出约 12.6 MP 的 `4096×3072` 深度图。使用 `--full-size` 时，`6000×4000` 图像会输出相同尺寸的深度图。无限远像素保存为 `+inf`。输入支持 JPEG、PNG、HEIC 以及 Pillow 能解码的其他静态图像格式。

使用 Python 3.11 或更新版本的虚拟环境。安装适配器依赖前，先安装适合硬件的 PyTorch wheel。[PyTorch 安装指南](https://pytorch.org/get-started/locally/)

```bash
python3 -m venv ~/.venvs/xdremux-depthpro2
source ~/.venvs/xdremux-depthpro2/bin/activate
python -m pip install -r model_adapters/depthpro2/requirements-pytorch.txt

python scripts/distribution/install_depthpro2.py --backend pytorch \
  --accept-apple-amlr-license --models s,b,fov
python model_adapters/depthpro2/pytorch_depth.py --model s \
  --input photo.heic --output depth.npy
python model_adapters/depthpro2/pytorch_depth.py --model b --full-size \
  --input photo_24mp.heic --output depth_full.npy
```

安装器把权重和许可凭据放到模型目录的 `pytorch` 子目录中，并验证 S、B 的官方 SHA-256 及共享 FOV 权重的文件大小。已知标定焦距时，可用 `--focal-length-px` 提供原图应用方向后的像素焦距。否则适配器优先使用 EXIF 里的 35 mm 等效焦距，缺少该值时按需加载 FOV 模型。

半尺寸模式按输出尺寸推理。`--full-size` 按原尺寸推理并保存全尺寸图。`--native` 按原尺寸推理并保存半尺寸图。这些模式默认都不额外缩小推理图。需要减小推理尺寸时，可以显式设置 `--max-side`。适配器会将逆深度缩放到所需的输出尺寸。降低长边上限可能使深度边缘变软。

处理大图时，主干网络保留全局注意力。解码器的最后几层采用带重叠区域的分条计算，为每条边界保留完整的像素邻域，从而降低同尺寸、同精度下的峰值内存。设备支持时使用融合注意力。PyTorch 2.13 起支持 MPS 长序列融合注意力。旧版 MPS 会分批计算查询，但保留所有键和值，耗时可能更长。实际耗时和内存取决于模型、设备、精度和补边后的尺寸。需要对齐到 32 像素时，适配器临时复制 RGB 图像的最下一行和最右一列，推理后将结果裁回推理图尺寸。

启用 oneDNN 的 CPU 会用原模型的权重和填充规则，将 FP32 分条解码头的转置卷积和紧随其后的卷积合并。合并后的卷积在上采样前执行，省去一张大型中间张量。运算仍采用 FP32，累加顺序的变化可能带来浮点舍入差异。

设备按 CUDA、MPS、CPU 的顺序自动选择，也可用 `--device` 和 `--precision` 覆盖。CUDA 优先用 BF16，否则用 FP16，MPS 使用 BF16，CPU 使用 FP32。按相同顺序重复传入 `--input` 和 `--output` 可批量处理独立图片，模型只加载一次。

### 为 Apple Portrait 选择 PyTorch

在安装命令中添加 `--bin-dir`，即可将启动器安装到已有的可执行文件目录。macOS 上可以为需要合成深度的 4:3 或 3:4 HDR 静态图像选择这个启动器。

```bash
XDREMUX_DEPTHPRO2_PYTORCH=/path/to/xdremux-depthpro2-torch \
XDREMUX_DEPTHPRO2_PYTORCH_MODEL=b \
  xdremux convert --input photo_4x3.heic --output portrait.heic --apple-portrait
```

两个模型都已安装时，启动器默认选择 S。用 `XDREMUX_DEPTHPRO2_PYTORCH_MODEL` 设为 `s` 或 `b` 可指定型号。源文件含有 OPPO 深度时，XDRemux 优先使用源深度。Portrait 转换仍依赖 macOS Vision 和 ImageIO 适配器生成语义遮罩和写入辅助资源。

## 能力与职责边界

- **视角焦距估计（`depthpro2-fov_512x896.mlpackage`）**：在 `xdremux-apple-adapter` 内对固定视角视频采样抽帧运行推理，在输入视频缺少已记录的 `FocalLengthIn35mmFormat` 元数据时，为 `xdremux convert --apple-cinematic` 输出标准的 `CinematicCameraEstimate` JSON（`schema_version: 1`）。
- **静态图视差估计（`depthpro2-b_w4.mlpackage` / `depthpro2-s_w4.mlpackage`）**：在 `xdremux-apple-adapter` 内通过 Core ML 与 Metal 提供逆深度到视差的推理原语，使 `xdremux convert --apple-portrait` 可以处理没有内置 OPPO `rear.depth` 的 4:3 与 3:4 ProXDR 或 ISO Gain Map 照片。
- **产品策略归属 Rust**：Swift 适配器仅负责 Core ML 推理、方向校正与 ImageIO/Vision 原语；Rust 负责相机观测准入、等效焦距决策、REND 生成、辅助图封装、结构校验及原子发布。

## 安装 Core ML 模型

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

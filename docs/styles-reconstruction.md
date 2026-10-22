# 摄影风格 HDR 重建

[English](styles-reconstruction.en.md) | 简体中文

`xdremux reconstruct-styles` 从原始 Apple HEIF 照片重建相对场景亮度。它保留原 SDR 主图的压缩数据、颜色属性和几何信息，用新的 RGB ISO Gain Map 承载 HDR 效果。SDR 效果保持不变。

此命令需要 macOS、Metal 和配套的 `xdremux-apple-adapter`。它使用 Apple 私有框架方法，适配器会在调用前检查方法签名。未来的系统版本可能使这项能力不可用。

## 输入与输出

输入必须包含与主图关联的 Styles 元数据、Style Delta Map 和 Linear Thumbnail。当前支持的元数据版本为 `0.8`、`0.9`、`0.15`、`0.16`。命令通过 HEIF 资源关系选择数据，不依赖固定 item 编号、文件名或相机型号。只有风格名称或预设名称不足以完成重建。版本未知、资源不完整、几何信息不一致或消费者检查失败时，命令会明确报错。

```bash
xdremux reconstruct-styles \
  --input original.HEIC \
  --output-dir reconstructed
```

重复 `--input` 可以批量处理。单张失败后继续处理其余输入，每张都有 JSON 结果。只要有一张失败，退出状态就是 `1`。

默认处理原始分辨率。使用 `--max-edge 1024` 可以限制处理分辨率及 Gain Map 的长边，同时限制诊断 EXR 的分辨率。此选项不会缩放或重新编码最终 HEIC 的 SDR 主图。

每张输入生成一个新的 `<source-stem>.reconstructed` 目录：

| 文件 | 用途 |
| --- | --- |
| `hdr.heic` | 保留原 SDR 主图，带重建后的 RGB ISO Gain Map。 |
| `scene_linear.exr` | 用于分析的相对扩展线性 Display P3 重建。 |
| `unstyled.exr` | 应用场景亮度约束之前的原生摄影风格反向处理结果。 |
| `report.json` | 输入和输出哈希、资源编号、原生框架事实、增益范围及回读检查。 |
| `globalToneCurveData.bin` | 原生解析器提供的色调曲线节点。 |
| `lightMapData.bin`、`linearLightMapData.bin` | 原生解析器提供的两张光照数据。 |

## 单独的 HDR 显示目标

使用 `--display-headroom 2`、`4` 或 `8`，可从重建的场景参考 R 生成显示目标 H。此时目录名为 `<source-stem>.display-<headroom>x.reconstructed`，增加 `display_linear.exr`，`hdr.heic` 承载 H。默认命令仍将 R 写入 `hdr.heic`。

```bash
xdremux reconstruct-styles --input original.HEIC --output-dir reconstructed --display-headroom 8
```

显示路径采用独立的中间调、皮肤亮度和对立色度方法，不提供苹果 Tone/Color 编辑坐标或二维调色盘。它从 R/SDR 的正亮度样本确定一个全局曝光比例，执行明确的压缩 `R * exposure * h / (h + Y(R * exposure))`，再除以 `h` 后应用有界算子。R 不会先被裁入编辑器的标称范围。颜色阶段参考同一照片扩展线性 SDR 的相对对立色度，保持影调阶段输出的亮度，并在该亮度下压缩色域。这些是本实现的明确选择，不代表完整复现苹果正向渲染器。

权重来自关联到主图的 2026 semantic-person 和旧版 2019 skin 遮罩。没有 semantic-person 时，可使用关联的 2018 Portrait Effects Matte，并明确记录为前景代理。缺失遮罩使用零权重；存在歧义则失败。天空和新版 skin 资源不会被静默替代。适配器不应用色彩管理或方向，在主图存储坐标中解码遮罩。引导图是本实现的面积亮度／对数高斯引导，不是苹果统计或引导滤波的复现。

报告记录系数、实际遮罩角色／URI／ID／哈希、曝光、有限且有界的浮点目标，以及原生 headroom 和回读。声明的显示容量与最大局部增益分别表达。HEVC 量化可能超出浮点界限、产生接近黑色的负值，或改变中间 headroom 的亮度。原生回读和 SDR 完全保留不能证明真实 HDR 观看质量。保留原图细节和纹理元数据，不重新运行纹理渲染器。机制和证据边界见[影调与颜色方法](research/styles-tonal-color-methods.md)。

EXR 像素使用显示方向。重建计算使用存储方向，以保持辅助资源与压缩 SDR 对齐。EXR 编码器可能使用半精度，报告记录实际回读误差。

已有输出目录不会被替换。同一输出目录下重复的输入文件主名会明确报错。报告最后写入。调用失败时只删除本次创建的目录，原照片始终只读。

## 重建与验证

1. Rust 提取关联的元数据及完整辅助图像子图，防止 ImageIO 通过替代图像组选择回其他主图。
2. 适配器向 Apple 获取反向配置与调参，依据原生纹理尺寸检查系数存储。Delta Map 根据其自身存储尺寸独立映射到主图画布。
3. Apple 反向处理器提供撤销摄影风格后的颜色，原 HDR 提供精细亮度细节。
4. Rust 用实测原生指数反函数 `2^((code - 0.69336945) / 0.08550479) - 0.00964052` 解码 Linear Thumbnail，计算有界的亮度增益场，再使用跟随主图亮度边缘的联合双边插值。原生 YCbCr 转 RGB 可能产生少量超出名义编码范围的数值；这不会触发另一套传递函数。
5. 一个稳健的全局标量保留原 HDR 的中间调曝光。运行时不使用 RAW 文件、逐机型拟合、按文件名设定的参数或训练模型。
6. Rust 相对于原 SDR 计算 RGB 增益。负增益可以压低 HDR 阴影。统一的 `1/64` 偏移量让黑位增益保持有限。Gain Map 使用 HEVC 4:4:4 编码，原压缩 SDR 图像树直接复制。
7. 派生 `tmap` 使用主图显示尺寸及恒等旋转。把竖拍主图的旋转再次复制给派生图像，会使 ImageIO 忽略 ISO Gain Map。
8. 成功前检查压缩 SDR 一致性、原生 SDR 回读、原生 HDR 展开、EXR 回读和输入完整性。SDR RGB 误差不得超过 `1e-6`；HDR 与目标的平均亮度误差不得超过 `0.15 EV`；EXR 平均亮度往返误差不得超过 `0.005 EV`。

生成的 HEIC 保留输入元数据和辅助资源。已有 Styles 资源仍描述原始拍摄。本命令不证明修改 Gain Map 后，Photos 内后续交互式摄影风格编辑仍与原文件等价。

## 限制

结果是相对场景亮度近似，不是 RAW 恢复，也不证明完整撤销了拍摄 ISP。内嵌缩略图的分辨率和精度有限，主图已经丢失的颜色和细节不能恢复。正值 HDR Gain Map 无法精确表达负线性颜色通道，因此 HEIC 输出会将它们裁到零，并在报告中记录比例。

色调曲线和两张光照数据解码后用于检查。这不等于已经确定完整的拍摄色调映射组合。运行时不会把两张光照图相除后就称为 ISP 逆变换。

当前路径在趾部也使用实测原生指数反函数，尚未证明它就是所有历史 Styles 文件的精确拍摄传递函数。未知元数据版本会明确拒绝。能力检测和回读检查不能替代更多场景的 RAW 对照。

此阶段不标定绝对辐亮度或漫反射白，尤其没有把漫反射白定义成 203 nits。HDR 显示自适应及真实 Photos／设备观感需要单独的消费者证据。

# 双层 HDR 转换

[English](dual-layer-hdr.en.md) | 简体中文

本页说明 XDRemux 当前支持的双层 HDR 静态照片转换、保留的源信息，以及尚未实现的工作。本页把已验证行为和计划行为分开描述。命令见 [CLI 参考](cli.md)。

## 支持的源文件

| 源文件 | 识别方式 | 说明 |
| --- | --- | --- |
| OPPO、OnePlus、realme 的 ProXDR HEIC | 厂商扩展块及其清单 | 本地 HDR 与 Ultra HDR 系列 |
| 独立 ISO 21496-1 双层 HEIF | 标准 `tmap` 图，且没有厂商扩展块 | 包括较早的 XDRemux HEIF 输出 |
| 双层 JPEG | `MPF` 索引与 ISO APP2 载体 | 只支持验证，不支持转换 |

Motion Photo 走 Live Photo 路径。普通单幅 JPEG 不是双层源文件。

## 支持的输出

| 输出 | 容器 | 派生图像声明 |
| --- | --- | --- |
| `compatible` | HEIF | BT.2020 PQ，沿用既有消费者布局 |
| `bt2020-pq` | HEIF | BT.2020 PQ |
| `bt2020-hlg` | HEIF | BT.2020 HLG |
| `.jpg` 或 `.jpeg` 目标 | 双层 JPEG | 基础色彩信息加 ISO 21496-1 增益映射参数 |

`batch` 只输出 HEIF。

## 转换保留的源信息

- 两条 HEIF 路径都保留压缩 SDR 基础图像的字节。JPEG 目标会解码后重新编码。
- 独立 ISO HEIF 转换为 HEIF 时，保留增益图的压缩分块、hvcC 字节、分块顺序、采样格式、位深与尺寸，并以源资源核对最终文件。生成的 10 位和 12 位 HEVC 向量覆盖此路径；不降低精度，也不要求增益图编码器。
- ProXDR 转换与 JPEG 目标使用既有的 8 位增益图编码器。JPEG 输出会重新编码两幅图像，采样不保证逐值相等。
- 增益映射处理元数据保留。独立 ISO 源保留原始色调映射载荷字节。ProXDR 源保留其厂商浮点视图声明的处理方式。
- 只有当源文件把 `mdcv`、`clli` 和注册 `it35` 载体关联到派生 HDR 图像时才复制。两类载体声明同一数值时必须一致。缺失的数值保持缺失。注册载体可选的矩阵和 3D LUT 扩展会经过边界校验后保留原始字节。
- JPEG 目标保留基础图像 ICC 配置文件与 Exif 载荷。

转换在发布前重新读取输出字节，检查结构、载体（包括显式启用的 APP8 声明实际偏移和增强载荷）与声明配置。JPEG 输出必须解码两幅图像。HEIF 重封装逐一比较压缩增益图资源，不解码或重新编码；像素解码是独立的验证操作。

JPEG 重建中，没有 alternate ICC 时，即使 `use_base_colour_space` 为零，仍在基本图像的应用色域中处理增益。存在的 alternate ICC 必须通过校验；若选作应用色域，使用其受支持的 BT.709、Display P3 D65 或 BT.2020 三基色执行线性增益操作，再转换到 BT.2020。不根据增益图的压缩矩阵推断应用色域。

## 映射与亮度执行器

两个纯执行器供已持有类型化输入的代码使用。它们都不读取文件。

`AdditionalMapping` 先应用三乘三矩阵，再应用可选的三维 LUT。矩阵在 `f64` 中先行，保留负的中间值。每个分量随后按 LUT 域归一化。节点按第一分量最快存储，包含该点的单元格八个节点按小数部分加权。域外数值默认拒绝，调用方选择时也可夹紧。固定向量覆盖中性白、非中性色、红轴最快的轴序、三节点边的内部采样，以及非单位域。

`AbsoluteDisplayTarget` 通过调用方声明的参考白，为相对线性 `1.0` 赋予亮度。高于峰值或低于黑位的分量会被夹紧并计数。`relative_linear_to_pq` 将标定后的亮度编码为 PQ。固定向量覆盖 100、203、1000 和 10000 cd/m²。

两个执行器都不会自行改变转换。注册载体中的映射扩展已能从文件解析并原样保留，但不能直接输入 `AdditionalMapping`：文件中的标量 LUT 没有声明该执行器所需的输出分量与域语义。显示目标也不会从设备读取。

因此，启用的注册基本图像映射或合成图像映射会阻止文件像素重建。结构验证和 HEIF 重封装仍可保留它。JPEG 导出若会丢弃启用的映射，必须选择 `--jpeg-registered-metadata`。JSON 验证分别报告基本图像映射、合成图像映射、动态元数据与显示适配；`parsed-not-applied` 不表示已经支持像素渲染。

`DynamicHdrMetadata::from_parts` 可打包调用方提供的统计值、最多两组显示目标参数、基曲线、样条段与饱和度增益，并检查每个字段的位宽和条件存在关系。该接口支持受校验的数据构造，不生成测量值或色调映射参数。

## 验证

在仓库根目录运行以下定向检查：

```bash
cargo test -p xdremux-runtime --test hdr_metadata_wiring
cargo test -p xdremux-runtime --test jpeg_export
cargo test -p xdremux-runtime --test jpeg_validation
cargo test -p xdremux-runtime --test jpeg_registered
cargo test -p xdremux-runtime --test iso_heif_source
cargo test -p xdremux-runtime --test hevc_gain_map_preservation
cargo test -p xdremux-cli --test jpeg_output
cargo test -p xdremux-cli --test iso_heif_source
cargo test -p xdremux-codec --test dual_jpeg
```

`xdremux validate` 报告输出文件：

```bash
xdremux validate output.heic --hdr-profile bt2020-hlg --semantic --decode-hdr --json
xdremux validate output.jpg --semantic --json
```

安装外部工具后，独立 ISO/JPEG 增益映射检查会运行。该检查要求独立读取器报告相同参数，并解码出完全相同的线性输出。工具缺失时检查会跳过并给出提示。

当前编码配置下的实测误差：

| 对比 | 平均 | 最大 |
| --- | --- | --- |
| JPEG 基础图像重编码相对源基础图像 | 低于 255 的 2 | 低于 255 的 24 |
| JPEG 输出相对同一源文件的 HEIF 输出 | 相对误差 0.19% | 第 99 百分位 1.3% |
| 通过压缩增益图重封装更换 HEIF 配置 | 0 | 0 |

这些数值对应一种固定编码配置，不是通用画质保证。

## 限制与待办

- 重建输出是相对线性 BT.2020。绝对亮度执行器需要调用方声明参考白；转换本身不提供。
- PQ 与 HLG 标记声明编码方式，不证明显示适配、色调映射或设备呈现。
- 动态元数据已解析为 PQ 编码的内容统计、目标显示参数、可选基曲线与样条参数以及饱和度增益，同时保留源字节。解析参数不等于显示适配，完整的目标设备色调映射算法仍未实现。
- JPEG 目标要求方向为单位变换，且 Exif 方向为正常值。
- JPEG 目标要求增益处理在基础应用色域中进行。声明替代色域的源文件会被拒绝。
- JPEG 目标不能携带显式 BT.2020 配置、OPPO 相机尾部数据或 Apple 编辑资源。
- 注册 JPEG APP8 配对载体只在显式请求（`--jpeg-registered-metadata`）且源文件提供增强图像注册元数据时写入。若源文件缺少该元数据，转换会拒绝生成，避免只写基础声明却缺少增强记录。简化视图偏移遵循仓库记录的规则，尚未由独立生产者样本确认。
- 增益图编码器目前接收 8 位光栅。独立 HEVC 增益图输出 HEIF 时通过压缩资源重封装保留精度；JPEG 目标显式选择 8 位量化，使用 JPEG 编码器，不要求 HEVC 增益图编码器。生成的单通道和 RGB 10/12 位源覆盖两类输出。
- HEIF 重封装目前要求分块编码尺寸一致、hvcC 字节一致，且增益图压缩声明与全范围 BT.601 相容。其他布局会拒绝，不会重新解释。改变替代应用色域需要经过验证的像素或映射转换，此重封装路径会拒绝该操作。
- 已支持从 HEIF `it35` 与 JPEG APP8 读取并往返保留注册矩阵/3D LUT 扩展，检查精度、有限值、长度及立方体维度。应用这些数据的渲染语义尚未确立；转换只保留数据，不改变像素，也不猜测 RGB 向量或 LUT 输入域。
- 本页不记录设备证据。图库识别、Photos 编辑与显示行为不在这些检查范围内。

## 代码归属

| 关注点 | Crate |
| --- | --- |
| 容器信息、载体提取、HEIF 组装 | `crates/xdremux-heif` |
| JPEG 索引、`MPF`、双图组装 | `crates/xdremux-format` |
| 源分析、转换、输出文件检查 | `crates/xdremux-runtime` |
| 请求、容器与配置策略 | `crates/xdremux-engine` |
| 解码、编码与增强图像分块 | `crates/xdremux-codec` |

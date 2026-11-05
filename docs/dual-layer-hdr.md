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
- 增强图像重新编码，保留解码采样与几何。
- 增益映射处理元数据保留。独立 ISO 源保留原始色调映射载荷字节。ProXDR 源保留其厂商浮点视图声明的处理方式。
- 只有当源文件把 `mdcv`、`clli` 和注册 `it35` 载体关联到派生 HDR 图像时才复制。两类载体声明同一数值时必须一致。缺失的数值保持缺失。
- JPEG 目标保留基础图像 ICC 配置文件与 Exif 载荷。

转换在发布前重新读取输出字节，检查结构、载体、声明配置，并解码两幅图像。

## 验证

在仓库根目录运行以下定向检查：

```bash
cargo test -p xdremux-runtime --test hdr_metadata_wiring
cargo test -p xdremux-runtime --test jpeg_export
cargo test -p xdremux-runtime --test jpeg_validation
cargo test -p xdremux-runtime --test iso_heif_source
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
| 经标准图路径更换配置 | 绝对误差 0.000002 | 绝对误差 0.003 |

这些数值对应一种固定编码配置，不是通用画质保证。

## 限制与待办

- 重建输出是相对线性 BT.2020。转换不建立绝对亮度、参考白或目标显示。
- PQ 与 HLG 标记声明编码方式，不证明显示适配、色调映射或设备呈现。
- 动态元数据只做语法检查并保留。其显示映射尚未实现。
- JPEG 目标要求方向为单位变换，且 Exif 方向为正常值。
- JPEG 目标要求增益处理在基础应用色域中进行。声明替代色域的源文件会被拒绝。
- JPEG 目标不能携带显式 BT.2020 配置、OPPO 相机尾部数据或 Apple 编辑资源。
- 不写入注册 JPEG APP8 载体。第二个偏移字段需要独立生产者样本才能正确实现。
- 拒绝额外矩阵与三维 LUT 映射。其载荷布局、节点顺序与插值契约尚未确定。
- 本页不记录设备证据。图库识别、Photos 编辑与显示行为不在这些检查范围内。

## 代码归属

| 关注点 | Crate |
| --- | --- |
| 容器信息、载体提取、HEIF 组装 | `crates/xdremux-heif` |
| JPEG 索引、`MPF`、双图组装 | `crates/xdremux-format` |
| 源分析、转换、输出文件检查 | `crates/xdremux-runtime` |
| 请求、容器与配置策略 | `crates/xdremux-engine` |
| 解码、编码与增强图像分块 | `crates/xdremux-codec` |

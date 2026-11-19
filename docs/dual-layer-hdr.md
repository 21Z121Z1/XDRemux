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

对于 ProXDR 静态照片，`batch` 输出 HEIF。

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

GB/T 48347—2026 采信 T/UWA 028.1—2024。表 16 和表 17 规定精度标志、九个矩阵系数和一个三维标量查找表，使用 16 位或 32 位浮点格式。第 10.2.3 和 10.2.6 节说明基本图像与提取的可选择图像应用映射的阶段。相关表格和过程未规定标量查找表的采样顺序、输入域、通道关联和插值规则。第 10 章规定按 GB/T 46269.1—2025 进行目标显示器的 HDR 显示适配。[静态图像标准](https://std.samr.gov.cn/gb/search/gbDetailed?id=5CB12E0EFB5B2034E06397BE0A0A34FB) [显示适配标准](https://openstd.samr.gov.cn/bzgk/std/newGbInfo?hcno=74075E2E20B1C2A3CDC429E6B51FA668)

GB/T 48347—2026 表 21 和表 23 规定两种 JPEG APP8 记录的提供者代码和提供者定向代码均为大端 16 位。表 21 的第一个偏移从基本图像的 APP8 标记开始计算，第二个偏移排除 JFIF 以外的 APP 数据。表 28 描述了 HEIF 的注册 `it35` 载荷。实现采用这些字段，并按输出布局检查两个 JPEG 偏移。完整的 APP8 互操作性验证仍需独立生产者给出能通过全部检查的文件。[静态图像标准](https://std.samr.gov.cn/gb/search/gbDetailed?id=5CB12E0EFB5B2034E06397BE0A0A34FB)

附录 C.5 展示了一项 `tmap`、两幅按序引用的源图像和一项未关联的 `it35`。文件只有一组以主图为基本图像的该结构，且只有一项没有引用、符合注册双图身份的元数据时，XDRemux 可以读取它。文件已有显式引用时按其关联读取。HEIF 写入器将保留的元数据写入一次，再用 `cdsc` 关联到新的 `tmap`。[静态图像标准](https://std.samr.gov.cn/gb/search/gbDetailed?id=5CB12E0EFB5B2034E06397BE0A0A34FB)

因此，启用的注册基本图像映射或合成图像映射会阻止文件像素重建。结构验证和 HEIF 重封装仍可保留它。JPEG 导出若会丢弃启用的映射，必须选择 `--jpeg-registered-metadata`。JSON 验证分别报告基本图像映射、合成图像映射、动态元数据与显示适配；`parsed-not-applied` 不表示已经支持像素渲染。

`DynamicHdrMetadata::from_parts` 可打包调用方提供的统计值、最多两组显示目标参数、基曲线、样条段与饱和度增益，并检查每个字段的位宽和条件存在关系。该接口支持受校验的数据构造，不生成测量值或色调映射参数。

回归测试采用 T/UWA 005.3-1—2023 表 A.2 的场景 6，按 GB/T 48347—2026 表 19 打包。测试检查显示目标、曲线参数、两个样条段和饱和度增益，再比较重新编码的字节。显示测试规定了早期视频标准的预期亮度计算方式。本用例验证元数据处理，像素适配仍待验证。[UWA 显示设备测试标准](https://www.uhd-world-association.com/wp-content/uploads/2024/10/UWA-_8-TUWA-005.3-1-2023-HDR-Video-Technology-Part-3-1-Technical-Requirements-and-Test-Methods-Display-Device.pdf)

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

Google libultrahdr v1.5.1 已用三星样本 `fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg` 通过此检查。它对原样本和重新封装的 JPEG 报告了相同的增益图参数，并将两者解码为逐字节一致的线性 RGBA 半精度浮点数据。该检查覆盖 ISO APP2 和 MPF。注册 APP8 和随目标显示器变化的 HDR 渲染仍待独立验证。[libultrahdr 源码](https://github.com/google/libultrahdr/tree/a8166d65171aef43cb4bc211538ee6619a9af680)

Google libultrahdr v2.0.2 独立读取了 `fixtures/proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic` 的兼容 HEIF 输出，以及以 `--hdr-profile bt2020-hlg` 重封装得到的 HEIF。`ultrahdr_app -m 1 -j FILE -P` 报告的增益图参数完全一致。两份文件经 `-o 0 -O 4` 解码得到逐字节一致的线性 RGBA 半精度浮点数据，SHA-256 为 `fd9d623d9387ad13cf01de275b3b8be5f990674794caf2fb97e424ea2313a151`。HLG 重封装文件也通过 XDRemux 的语义和 HDR 解码校验。[libultrahdr v2.0.2 源码](https://github.com/google/libultrahdr/tree/e5f5a022fe96fc4dc2ee35c19f733a50df807abe)

同一外部编码器生成的两份 96×64 ISO HEIF 通过了 XDRemux 的 `validate --semantic`。基图 `nclx` 分别为 `[1, 1, 1]` 和 `[9, 1, 9]`。显式 BT.2020 重封装均在写入前停止，因为该配置要求基图使用带 sRGB 传递函数的 sRGB 或 Display P3 编码。第一份样本 SHA-256 为 `9bf210138313a49563207eb89b54801352a7878e93b96b665716110a6a8505a3`，BT.2020 样本 SHA-256 为 `8e6de139ad975e796583a96e8c3ee4dc04fc694303e717ce6ec9778d58430139`。[HEIF 编码器 API](https://github.com/google/libultrahdr/blob/e5f5a022fe96fc4dc2ee35c19f733a50df807abe/docs/api_changes_v1.5_to_v2.0.md)

另检查了公开相机原图的 JPEG 注册载体。DCFever 的两份原图早于 GB/T 48347—2026 在 2026 年 9 月发布。Fotopolis 的两份原图拍摄于同年 10 月。

| 相机原图 | 文件 SHA-256 | 观察到的布局 |
| --- | --- | --- |
| 华为 Pura 90 Pro Max，DCFever 33253 | `75dede756cbce0026d6e505c233c00451b1b985f46fb527b9cdb562b1a86ec3c` | 两幅图均使用 `ITUT35` APP8，提供者定向代码为 `0x0007`。原始偏移指向增强图像 SOI。第二偏移排除了 28 字节的 APP8 及紧随其后的 18 字节 JFIF APP0。两幅 JPEG 后有 `VLDMAGIC` 不透明尾部。[相机原图](https://www.dcfever.com/phones/viewsamples.php?picture=33253) |
| 华为 Pura 80 Ultra，DCFever 31923 | `a26adb6f4cddff91f3e8196cfe276d27f0a34836fab4c960c2aa9283cc4f17e3` | 两幅图均使用 `ITUT35` APP5，提供者定向代码为 `0x0006`。[相机原图](https://www.dcfever.com/phones/viewsamples.php?picture=31923) |
| 华为 Pura 90s Pro Max，Fotopolis 264014、264011 | `217189324b3de28da1859a84fb137c1ed5a2c678110f348b44437b124928abaa`、`4a65f9c75cbc9dbb9d1e5bc252d5c6e3c81cd7aae731dd151b82b2a354cb40f7` | 两份原图的两幅 JPEG 均使用 `ITUT35` APP5 和 `0x0006`。均未发现 ISO APP2 或 MPF。以第一处 APP5 标记为起点，原始偏移到达第二幅 JPEG 的 APP5 标记。[相机原图](https://www.fotopolis.pl/opinie/recenzje/39939-fotograficzne-mozliwosci-ktore-zmiescisz-w-kieszeni-sprawdzamy-aparaty-huawei-pura-90s-pro-max-w-podrozy) |

Libultrahdr 通过 ISO APP2 将 Pura 90 原图识别为 Ultra HDR。XDRemux 的规范格式校验器拒绝该文件的尾部。只取两幅 JPEG 后，校验器仍拒绝第二偏移。对本地诊断副本修正偏移后，发现母版显示记录先列三项 x 坐标，再列三项 y 坐标。按表 18 交错排列坐标后，校验器报告动态载荷尾部或填充异常。原图保存在仓库外。ISO 读取通过和 APP8 头部一致只验证了各自的读取范围，注册载荷互操作和渲染仍待验证。

OpenHarmony 提供了独立测试样本和载体读取源码。其 JPEG 读取器用第一个 APP8 偏移定位增益图，并将第二个偏移写入日志。第二个偏移仍需独立校验。下表使用 OpenHarmony 提交 `d4fd420d088aacb7401ccb7d7d5bce1ad3967e56` 的原始样本。[OpenHarmony JPEG 读取器](https://github.com/openharmony/multimedia_image_framework/blob/d4fd420d088aacb7401ccb7d7d5bce1ad3967e56/plugins/common/libs/image/libextplugin/src/hdr/hdr_helper.cpp#L159-L199)

| OpenHarmony 样本 | 文件 SHA-256 | XDRemux 结果 |
| --- | --- | --- |
| `HdrVivid.jpg` | `39280746144cd5af36906f57902226de2a112efcae82cae147602d48029b1bfc` | 两幅图的 `ITUT35` APP8 都使用 `0x0007`。原始偏移到达第 824420 字节。MPF 给出的增强图像起点为第 824440 字节。[样本](https://github.com/openharmony/multimedia_image_framework/blob/d4fd420d088aacb7401ccb7d7d5bce1ad3967e56/test/resource/image/images/HdrVivid.jpg) |
| `hdr_media_type_test.jpg` | `ee95ce874d4a9a105301d5cdb3c8984ec4a651d6ed9cfe9cc5cd73691a743b9e` | 两个 APP8 偏移均符合成品的图像边界。该文件声明 28 字节的静态记录，但所有原色和白点坐标为零，HDR 元数据校验拒绝该数据。[样本](https://github.com/openharmony/multimedia_image_framework/blob/d4fd420d088aacb7401ccb7d7d5bce1ad3967e56/test/resource/image/images/hdr_media_type_test.jpg) |
| `test_heif_hdr.heic` | `0c76cd66ca775fc348558b54452888d603efd418b5070392b35550a97baaa931` | HEIF 增益图结构校验通过。第 28 项通过 `cdsc` 关联到 `tmap` 第 27 项。独立调用 HDR 元数据读取时因母版色度被拒绝。独立调用 ISO 语义校验时因 `gainMapMin[0]` 分母为零被拒绝。[样本](https://github.com/openharmony/multimedia_image_framework/blob/d4fd420d088aacb7401ccb7d7d5bce1ad3967e56/test/resource/image/images/test_heif_hdr.heic) |

分别从两份 JPEG 样本取出原始 53 字节动态区段，省略静态记录后单独解码。两份样本都读出两个目标 PQ 码值（3078 和 2080）、每个目标的两段样条，以及两项饱和度增益。用解析字段重新打包，能分别逐字节重建原区段。该检查独立验证了动态元数据的编解码，像素适配仍待验证。

OpenHarmony 的 `Picture` 实现将 HDR 合成交给运行时加载的平台 VPE 函数。独立设备验证可导入带注册载体的原片及重封装结果，通过 Image Kit 读取各自的增益图和合成 HDR 像素，然后在两个已校准的 HDR 显示目标上测量原片和输出以验证适配。记录系统构建、峰值亮度、传递函数及像素或亮度计实测数据。UWA 通过申请表向会员提供参考代码。[OpenHarmony 合成调用](https://github.com/openharmony/multimedia_image_framework/blob/d4fd420d088aacb7401ccb7d7d5bce1ad3967e56/frameworks/innerkitsimpl/utils/src/vpe_utils.cpp#L134-L156) [华为 Image Kit 指南](https://developer.huawei.com/consumer/en/doc/HarmonyOS-Guides/image-hdr-decoding-c/) [UWA 参考资源](https://theuwa.com/tech/faq_en.html)

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
- 该选项写入注册元数据载体。JPEG 写入器沿用源基本图像已有的 Exif 和 ICC，省略提取的可选择图像的 ICC。GB/T 48347—2026 第 9.2 节规定的完整分发格式符合性仍待验证。
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

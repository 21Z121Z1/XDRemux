# HDR Log Linear Thumbnail 验证

简体中文 | [English](20261014-hdr-log-thumbnail.en.md)

本次改动将 Styles Linear Thumbnail 中直接复制的 SDR 码值，替换为对重建的线性 Display P3 HDR 应用 Apple Log 曲线的结果。曝光归一化使用 Styles plist key `4`。现有 key1 生成策略和辅助图像色彩标记继续沿用。

## 编码资源回归

原生回归对两张已入库的 ProXDR 夹具运行 Rust Styles 转换，通过 libheif 回读 VideoToolbox 编码的 Main10 缩略图，对照未压缩的 HDR Log 目标，并检查主图 HEVC 有效载荷及配置保持不变。误差单位为 8-bit RGB 码值。

| 夹具 | 回读缩略图 | 对 HDR Log 目标的 MAE | 对 SDR 图像的 MAE |
| --- | --- | ---: | ---: |
| `oppo/find-x9-ultra/uhdr-portrait-01.heic` | 770 × 1024 | 1.5852 / 255 | 32.9170 / 255 |
| `oppo/find-x7-ultra/lhdr-v2-01.heic` | 1024 × 878 | 0.9890 / 255 | 42.3492 / 255 |

UHDR 夹具覆盖 RGB gain，LHDR 夹具覆盖单色 gain。接收门槛为对 HDR Log 目标的 MAE 小于 3 / 255；检查也会拒绝仍类似 SDR 复制图的缩略图。单元测试覆盖 Log 趾部、超过 SDR 白的数值、曝光归一化、通道顺序、几何关系和无效输入拒绝。

反向对照临时恢复原来的 SDR 复制语句后，同一真实媒体测试在 UHDR 夹具上以 HDR Log MAE 32.0010 / 255 失败。最终构建和 completion gate 运行前已恢复修正语句。

当前编码原语接受 RGB8 输入，输出 Main10 4:2:0。这些结果不代表无损 10-bit RGB 重建。

## 同条件原生渲染诊断

基线文件只恢复了原来的 SDR 复制语句，修正文件使用新语句。夹具、adapter、编码设置和 key1 策略相同。所有 item 的有效载荷中，仅缩略图改变；UHDR 其余 103 个和 LHDR 其余 115 个 item 的有效载荷逐字节相同，包括 Styles 属性列表。编码器提供与缩略图匹配的 HEVC 配置。

诊断覆盖 disabled、neutral、Tone −1、Tone +1、Color −1、Color +1、Tone −1 / Color +1 和 Tone +1 / Color +1。设置为 Standard cast、intensity 1、最大尺寸 1024，共成功生成 32 张原生渲染。

在 macOS 27.0.1 build 26A434 上，Neutrino helper 对每个文件的所有设置都生成了相同像素，基线和修正版之间也逐像素相同。helper 报告了请求的设置和 PNG 成功写出，但本次结果不能证明 Styles 调整实际生效。当前产品输出 version 15 Styles 元数据，此前有响应的研究载体使用 version 16。因此，这组对照未证明当前产品载体的 Tone 或 Color 存在可见改善。

helper SHA-256 为 `476a36a38a88f6860c8dbb6c21133c1c40e1d0dc565a4c257c03e632be51afd7`。该结果与此前 RAW 参照误差、独立的完整 key1 求解实验分开记录；后两者均不是本 PR 的接收指标。

## 复现与边界

运行原生资源回归及现有 Styles consumer 检查：

```bash
bash scripts/apple/check_rust_style_consumer.sh
```

脚本会将构建出的 Apple adapter 传给新 Rust 测试。没有 adapter 的可移植测试会跳过该项；macOS consumer gate 会提供 adapter 并实际运行。

本次证据支持编码 HDR 参照得到修正、主图 HEVC 和 Styles 元数据保持不变。它不证明 RAW 恢复、相机原生 producer 状态、完整转换时间降低或 Photos 导入／编辑／保存／重开。Photos 和设备 HDR 显示仍需单独验证。

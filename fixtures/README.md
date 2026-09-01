# 真实媒体 Fixture

[English](README.en.md) | 简体中文

本目录保存用于验证 XDRemux 可移植 Rust 实现及迁移期 oracle 的真实设备原始媒体输入。所有样张都应保持原始字节不变。

## 目录结构

- `motion-photo/<vendor>/...`：Android Motion Photo 样张，覆盖 JPEG 与 HEIC/HEIF 布局。
- `proxdr/<vendor>/<device>/...`：厂商原始 ProXDR HEIC，用于验证 HDR 提取、family 判定、Gain Map 重建、HEIF 组装和 CLI 转换。

路径属于 fixture 契约的一部分。命名优先描述稳定的能力或布局，而不是拍摄时间。厂商/设备目录只记录测试输入来源，不是产品支持白名单。

## 文件身份契约

媒体文件按原始字节提交。`SHA256SUMS` 是本目录所有版本化真实媒体 fixture 的统一 canonical identity manifest。

严格 real-fixture gate 必须拒绝字节与记录 digest 不一致的文件。如果 metadata、方向、容器布局、厂商尾数据或内嵌资源属于被测行为，不要为了“规范化”而改写源 fixture。

## Fixture 保留的数据

真实 fixture 可能包含 EXIF、拍摄时间、厂商 metadata、内嵌 Motion Video、HDR Gain Map、人像数据、方向数据、Local HDR metadata，以及 parser/validator 所需的其他原始 payload。这些都属于测试输入的一部分。

不要因为文件位于 `fixtures/` 下，就假定真实照片已经去敏。

## 当前覆盖范围

Motion Photo corpus 当前包含 Samsung、Xiaomi、OPPO、vivo 样张，并覆盖 JPEG 与 HEIC/HEIF 容器。

ProXDR corpus 当前包含 OPPO Find X6 Pro 的 LHDR v1、Find X7 Ultra 的 LHDR v2（含 XPAN），以及 Find X9 Ultra 的 UHDR 样张（含高分辨率、Master 和 Portrait）。这些 fixture 用于暴露格式与产品策略差异，并不意味着每台设备的所有拍摄模式都已经得到覆盖。

## 生成输出

生成的 ISO HDR HEIC 和 Live Photo HEIC/MOV 都是临时测试或 workflow artifact。除非未来测试明确把某个输出定义为版本化 golden artifact，否则不要把转换结果提交到这里。

## 增加 fixture

增加真实 fixture 时：

1. 确认文件可以公开提交。
2. 不修改字节，提交原始文件。
3. 放入对应的能力/厂商/设备层级。
4. 在 `SHA256SUMS` 中加入 SHA-256 digest。
5. 增加或更新测试，明确这个 fixture 能证明什么。
6. 不要根据一个 fixture 推断无关的设备支持。

仓库测试政策见 [docs/quality/testing.md](../docs/quality/testing.md)。

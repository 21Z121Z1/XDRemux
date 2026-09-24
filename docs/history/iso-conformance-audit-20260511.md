# XDRemux ISO Conformance Audit Report

**Date**: 2026-05-11
**Scope**: Python (`python/`) + Swift (`swift/XDRemux.swift`) HEIC 输出
**Standards**:
- ISO 21496-1:2025 — HDR Gain Map
- ISO/IEC 23008-12:2025 — HEIF Container
- ISO/IEC 23008-12:2025 Amd 1 — Tone Map Derived Image (`tmap`)

---

## Executive Summary

| Severity | Count | Description |
|----------|-------|-------------|
| RESOLVED | 2 | Python hdrgm XMP 已嵌入；Swift HDRCapacity 映射已修复 |
| COMPLIANT | 9 | gain map colr、ftyp brands、dimg、altr、hidden flags、bit depth、orientation 等 |
| SHOULD GAP | 1 | 当前 gain map 尺寸为半分辨率，不等于 base；ISO 21496-1 §4.2 为 SHOULD，默认 CI 不阻断，strict 检查可报告 |
| COMPAT CHOICE | 1 | Python 默认保留 Apple 可识别 HDR 的 62B tmap payload；严格 ISO GainMapMetadata 作为参考 helper/未来模式 |

---

## 1. COMPAT CHOICE: Python tmap Payload 保留 Apple HDR 兼容

**文件**: `python/isobmff_patch.py:198-232` (`_build_tmap_config`)

### 当前实现

Python 默认生成/保留 62 字节 Apple ImageIO 风格 payload。该格式能触发 Apple CoreImage Headroom，是当前产品目标的优先约束；它不是严格 ISO/IEC 23008-12 Amd 1 `ToneMapImage` + ISO 21496-1 C.2.2 `GainMapMetadata` 二进制布局。

验证结果：
- normal 输出: `check_hdr.swift /tmp/xdremux_apple_normal.heic` Headroom=`2.4211792945861816`
- passthrough 输出: `check_hdr.swift /tmp/xdremux_apple_passthrough.heic` Headroom=`2.4211792945861816`

### 严格 ISO 参考格式

HEIF `tmap` item payload 现在为：

```
Offset  Size  Field
0       1B    ToneMapImage.version = 0
1-4     4B    GainMapVersion: minimum_version=0, writer_version=0
5       1B    flags: bit7=is_multichannel, bit6=use_base_colour_space
6-8     3B    reserved = 0
9-16    8B    base_hdr_headroom rational
17-24   8B    alternate_hdr_headroom rational
25..    40B/channel GainMapChannel
```

Total: **65 bytes** (1ch) or **145 bytes** (3ch)。这是 1 字节 `ToneMapImage.version` 加上 64/144 字节 ISO 21496-1 `GainMapMetadata`。

仓库保留 `_build_iso_gainmap_metadata_payload()` 作为严格 ISO metadata reference helper，`evals/test_eval.py` 覆盖 64/144 字节结构与 multichannel flag。默认写入路径暂不使用它，以避免破坏 Apple HDR 识别。

### ISO 21496-1 C.2.2 GainMapMetadata

```
Offset  Size  Field
0-1     2B    minimum_version (uint16 BE) = 0
2-3     2B    writer_version (uint16 BE) = 0
4       1B    flags: bit7=is_multichannel, bit6=use_base_colour_space, bits5-0=reserved
5-7     3B    reserved (padding)
8-11    4B    base_hdr_headroom_numerator (int32 BE)
12-15   4B    base_hdr_headroom_denominator (uint32 BE)
16-19   4B    alternate_hdr_headroom_numerator (int32 BE)
20-23   4B    alternate_hdr_headroom_denominator (uint32 BE)
--- per-channel (1ch = 40B, 3ch = 120B) ---
+0-3    4B    gain_map_min_numerator (int32 BE)
+4-7    4B    gain_map_min_denominator (uint32 BE)
+8-11   4B    gain_map_max_numerator (int32 BE)
+12-15  4B    gain_map_max_denominator (uint32 BE)
+16-19  4B    gamma_numerator (uint32 BE)
+20-23  4B    gamma_denominator (uint32 BE)
+24-27  4B    base_offset_numerator (int32 BE)
+28-31  4B    base_offset_denominator (uint32 BE)
+32-35  4B    alternate_offset_numerator (int32 BE)
+36-39  4B    alternate_offset_denominator (uint32 BE)
```

Total: **64 bytes** (1ch) or **144 bytes** (3ch)，作为 HEIF `ToneMapImage` 的 `gain_map_metadata[]` 字段存储。

### 验证

`test_heic_parser.py` 默认接受 Apple 62B payload，并继续验证 tmap/dimg/XMP/bit-depth/orientation 等外层结构。`--strict-iso-tmap` 可用于实验性严格 ISO payload 检查。

### 兼容性风险

本机 CoreImage `check_hdr.swift` 对严格 ISO payload 输出曾返回 Headroom=1.0 或无法加载 passthrough 输出。因此当前策略是 Apple HDR 优先、ISO 尽量靠近：保留 Apple tmap payload，同时补齐 hdrgm XMP、Swift 字段映射、容器结构检查和 ISO metadata reference helper。

---

## 2. RESOLVED: Swift XMP HDRCapacity 字段映射错误

**文件**: `swift/XDRemux.swift:1393-1394`

### 现状

```swift
try set("hdrgm:HDRCapacityMin", formatFloat(style.gainMapMin, digits: 6) as CFString)
try set("hdrgm:HDRCapacityMax", formatFloat(style.gainMapMax, digits: 6) as CFString)
```

### 修复后

```swift
try set("hdrgm:HDRCapacityMin", formatFloat(style.baseHeadroom, digits: 6) as CFString)
try set("hdrgm:HDRCapacityMax", formatFloat(style.alternateHeadroom, digits: 6) as CFString)
```

### 问题

`HDRCapacityMin/Max` 被错误地映射到 `gainMapMin/gainMapMax`：

| XMP Field | Swift 映射 | 正确含义 |
|-----------|----------|---------|
| `hdrgm:HDRCapacityMin` | `style.gainMapMin` | 应为 `hdrCapacityMin`（最小显示 headroom） |
| `hdrgm:HDRCapacityMax` | `style.gainMapMax` | 应为 `hdrCapacityMax`（最大显示 headroom） |

- `gainMapMin` = gain map 像素最小值（log2 域），通常 = 0.0
- `gainMapMax` = gain map 像素最大值（log2 域），通常 = 1.0~3.0
- `hdrCapacityMin` = 最低支持 HDR 的显示器 headroom，通常 = 0.0
- `hdrCapacityMax` = 目标显示器最大 headroom，通常 ≈ gainMapMax

### 对比

Python 版本 `iso21496.py` 正确区分了这些字段：
```python
"hdrCapacityMin": cap_min,   # 来自 displayRatioSdr
"hdrCapacityMax": cap_max,   # 来自 displayRatioHdr
"gainMapMin": gm_min,        # 来自 ratioMin
"gainMapMax": gm_max,        # 来自 ratioMax
```

### 影响

在大多数单增益图场景下，`gainMapMax ≈ hdrCapacityMax`，所以影响较小。但在以下场景会出错：
- `hdrCapacityMin > 0` 的场景（显示器需要最低 headroom）
- `gainMapMax ≠ hdrCapacityMax` 的场景（如 gain map 被裁剪）

---

## 3. RESOLVED: Python hdrgm XMP 已嵌入文件

**文件**: `python/iso21496.py:120-155` (构建), `python/heif_io.py` (未调用)

### 现状

`iso21496.format_hdrgm_xmp()` 函数构建了完整的 hdrgm XMP 文档：

```xml
<x:xmpmeta xmlns:x="adobe:ns:meta/">
   <rdf:RDF>
      <rdf:Description xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/">
         <hdrgm:Version>1.0</hdrgm:Version>
         <hdrgm:GainMapMin>0.0 0.0 0.0</hdrgm:GainMapMin>
         <hdrgm:GainMapMax>...</hdrgm:GainMapMax>
         <hdrgm:Gamma>1.0 1.0 1.0</hdrgm:Gamma>
         <hdrgm:OffsetSDR>0.0 0.0 0.0</hdrgm:OffsetSDR>
         <hdrgm:OffsetHDR>0.0 0.0 0.0</hdrgm:OffsetHDR>
         <hdrgm:HDRCapacityMin>0.0</hdrgm:HDRCapacityMin>
         <hdrgm:HDRCapacityMax>...</hdrgm:HDRCapacityMax>
         <hdrgm:BaseRenditionIsHDR>False</hdrgm:BaseRenditionIsHDR>
      </rdf:Description>
   </rdf:RDF>
</x:xmpmeta>
```

两个 Python 写入路径现在都会写入 XMP metadata item：
- `heif_io.write_heic()` 通过 `isobmff_patch.patch_heic_for_iso21496()` 添加 `mime` item。
- `heif_io.write_heic_passthrough()` 在重建 `iinf`/`iloc`/`idat`/`iref` 时添加 `mime` item。

XMP item 使用 `item_type='mime'`、`content_type='application/rdf+xml'`，payload 为 `iso21496.format_hdrgm_xmp()` 输出，并通过 `cdsc` 同时关联 base grid 与 tmap item。

### 验证

`test_heic_parser.py` 现在要求存在 `application/rdf+xml` 的 `mime` item，并要求其 `cdsc` 指向 `tmap` item。

### Swift 对比

Swift 版本正确嵌入了两种 XMP：
1. `hdrgm:*` XMP — 通过 `kCGImageDestinationMergeMetadata` 合并到主图 metadata
2. `HDRToneMap:*` XMP — 通过 `kCGImageAuxiliaryDataInfoMetadata` 作为 gain map 辅助数据

---

## 4. COMPLIANT: gain map colr nclx 属性

**文件**: `python/isobmff_patch.py:105-112`

### 现状

```python
COLR_NCLX_SRGB_BOX = (
    b'\x00\x00\x00\x13'     # size = 19
    b'\x63\x6f\x6c\x72'     # type = "colr"
    b'\x6e\x63\x6c\x78'     # colour_type = "nclx"
    b'\x00\x02'              # colour_primaries = 2 (sRGB)
    b'\x00\x02'              # transfer_characteristics = 2 (sRGB)
    b'\x00\x02'              # matrix_coefficients = 2 (sRGB)
    b'\x80'                  # full_range_flag = 1
)
```

### ISO 23008-12 Amd 1 §6.6.2.4.1 要求

> The gain map input image item SHALL be associated with a 'colr' item property of type 'nclx':
> - colour_primaries SHALL be set to 2
> - transfer_characteristics SHALL be set to 2
> - full_range_flag may be set to either 1 or 0

### 结论: COMPLIANT

---

## 5. COMPLIANT: tmap colr 属性

**文件**: `python/isobmff_patch.py:93-101`

### 现状

```python
COLR_NCLX_PQ_BOX = (
    b'\x00\x00\x00\x13'     # size = 19
    b'\x63\x6f\x6c\x72'     # type = "colr"
    b'\x6e\x63\x6c\x78'     # colour_type = "nclx"
    b'\x00\x09'              # colour_primaries = 9 (BT.2020)
    b'\x00\x10'              # transfer_characteristics = 16 (PQ / ST 2084)
    b'\x00\x09'              # matrix_coefficients = 9 (BT.2020 NCL)
    b'\x80'                  # full_range_flag = 1
)
```

### ISO 23008-12 Amd 1 §6.6.2.4.1 要求

> A 'tmap' derived image item SHALL be associated with a 'colr' item property. This corresponds to the alternate image colorimetry metadata described in ISO 21496-1.

tmap 的 colr 描述 alternate (HDR) image 的色彩空间。BT.2020 + PQ 是 HDR 内容的标准色彩空间。

### 结论: COMPLIANT

---

## 6. COMPLIANT: ftyp brands

**文件**: `python/isobmff_patch.py:906-911`

### 现状

添加到 compatible_brands: `tmap`, `MiHE`, `MiHB`

### ISO 23008-12 Amd 1 §10.2.6.1 要求

> When a tone-map derived item is present, this brand SHALL be among the brands included in the compatible brands array of the FileTypeBox.

### 结论: COMPLIANT

`tmap` brand 已添加。`MiHE`/`MiHB` 是小米自定义 brand，不影响合规性。

---

## 7. COMPLIANT: dimg references

**文件**: `python/isobmff_patch.py:780-791`

### 现状

```
dimg: tmap → [primary_grid, gainmap_grid]  (reference_count = 2)
dimg: primary_grid → [primary_hvc1]        (reference_count = 1)
dimg: gainmap_grid → [gainmap_hvc1]        (reference_count = 1)
```

### ISO 23008-12 Amd 1 §6.6.2.4.1 要求

> reference_count SHALL be equal to 2
> the first SHALL be the base input image and the second SHALL be the gain map input image

### 结论: COMPLIANT

---

## 8. COMPLIANT: grpl/altr

**文件**: `python/isobmff_patch.py:828-836`

### 现状

写入 altr group，包含 tmap 和 primary_grid items。

### ISO 23008-12 Amd 1 (推荐)

> Backwards compatibility with parsers that do not support the tone-map derivation can be achieved by placing the base input image item and the 'tmap' derived image item in an 'altr' entity group.

### 结论: COMPLIANT (推荐做法已实现)

---

## 9. COMPLIANT: Item hidden flags

**文件**: `python/isobmff_patch.py:759-769`

### 现状

| Item | Type | Hidden Flag | Role |
|------|------|-------------|------|
| Primary hvc1 | hvc1 | 1 (hidden) | 原始编码 base image |
| Gainmap hvc1 | hvc1 | 1 (hidden) | 原始编码 gain map |
| Primary grid | grid | 0 (visible) | 主图 grid wrapper (pitm) |
| Gainmap grid | grid | 1 (hidden) | 增益图 grid wrapper |
| tmap | tmap | 0 (visible) | Tone map derived item |

### ISO 23008-12 要求

- §6.4.2: The primary item SHALL NOT be a hidden image item ✓
- Amd 1 §6.6.2.4.1: The gain map input image should be marked as hidden ✓
- §6.4.2: altr groups SHALL NOT mix hidden and non-hidden items ✓ (tmap + primary_grid 都是 visible)

### 结论: COMPLIANT

---

## 10-12. VERIFIED: 实际文件验证

### 10. 增益图尺寸一致性

**ISO 21496-1 §4.2**: gain map 尺寸 SHOULD 等于 baseline image 尺寸
**ISO 21496-1 §4.2**: gain map 尺寸 SHALL 在 metadata (ispe) 中声明

结论：**SHOULD GAP**。当前 Python normal 与 passthrough 输出均声明了 gainmap `ispe`，但样张输出为半分辨率 gainmap：

- base: `3072x4096`
- gainmap: `1536x2048`

`test_heic_parser.py` 默认不阻断 SHOULD 级尺寸不一致；使用 `--strict-should` 会报告该缺口。

### 11. 增益图 bit depth

**ISO 21496-1 §4.4**: gain map bit depth SHALL ≥ 8 bits

结论：**COMPLIANT**。`test_heic_parser.py` 验证 gainmap input item 关联 `pixi`，且所有 channel bit depth 均 >= 8。

### 12. 增益图方向

**ISO 21496-1 §4.5**: gain map 方向 SHALL 匹配 baseline image 方向

结论：**COMPLIANT**。`test_heic_parser.py` 验证 tmap 的 base input 与 gainmap input 均关联 `irot`，且 angle 一致。

---

## 附录: 检查方法

### 生成测试文件
```bash
# Python normal mode
python3 -m python.XDRemux convert --input test-samples/xxx.heic

# Python passthrough mode
python3 -m python.XDRemux convert --input test-samples/xxx.heic --passthrough

# Swift version
swift swift/XDRemux.swift convert --input test-samples/xxx.heic
```

### 运行验证器
```bash
python3 test_heic_parser.py output.heic
```

### 检查 tmap payload
```bash
python3 -c "
import struct
data = open('output.heic', 'rb').read()
# Find tmap item in idat, extract 62 bytes, dump hex
"
```

### 检查 XMP
```bash
exiftool -xmp -b output.heic
```

### CoreImage HDR 检测
```bash
swift check_hdr.swift output.heic
```

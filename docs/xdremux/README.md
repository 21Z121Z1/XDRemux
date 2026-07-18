# XDRemux 文档

用途: 聚合 XDRemux 转换器、ISO/HEIF 合规、Apple ImageIO 兼容和 passthrough 相关文档。

## 当前优先读取
1. [ISO compliance report v2](iso-compliance-report-v2-20260514.md): 最新完整审计。v2 修正了 v1 中 passthrough base `colr`/`irot` 误报，当前无 SHALL 失败。
2. [ISO conformance audit 2026-05-11](iso-conformance-audit-20260511.md): 早期审计和 Apple 62B tmap 兼容策略说明。
3. [ISOBMFF patcher progress](isobmff-patcher-progress-20260507.md): Python patcher 让 CoreImage Headroom 可识别的闭环记录。
4. [Passthrough plan](passthrough-plan.md): Python passthrough 设计背景。

## 过期或低优先级
- `docs/archive/xdremux/iso-compliance-report-v1-20260513.md` 已被 v2 覆盖，只保留用于对比修复前后的误报。
- `iso-conformance-audit-20260511.md` 中的待办项已部分被 v2 和 eval 覆盖；不要直接拿它判断当前状态。

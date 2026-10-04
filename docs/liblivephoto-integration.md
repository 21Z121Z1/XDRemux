# LibLivePhoto 接入

[English](liblivephoto-integration.en.md)

LibLivePhoto 负责可移植的 Motion/Live Photo 文件格式契约。XDRemux 通过
其资源 facade 完成源检查、分类、主资源选择、精确时间和 pair 验证。
Cargo manifest 和 lockfile 用同一完整 Git revision 固定两个独立库依赖。
LibLivePhoto 可以在 XDRemux workspace 外构建。

`xdremux-format` 重新导出 `liblivephoto-format` 的 cursor、error、FourCC、
JPEG、原始 EXIF 和 BMFF 基础类型。产品 codec profile 检查仍在本地。
`xdremux-motion-photo` 保留产品几何策略、文件复制和原子 pair 发布。
旧名称通过不稳定的迁移 feature 重新导出。新格式操作应使用 `MotionPhoto`、
`Input`、资源和 `MediaTime`，不得在 consumer crate 增加另一套 parser。

runtime 保留已知的整数和 timescale 时间，不再把已知 XMP 时间舍入到视频帧。
源时间未知时，现有产品 fallback 选择一个位置，并在回执中标记为推断。
Apple 几何元数据接收显式调用者值。OPPO 矩阵和防抖策略仍归 XDRemux 所有。

JPEG 和 Gain Map 编码属于产品操作。回执报告精确时间、视频 payload 字节比较、
元数据改写、重编码需求、新 identity 以及未进入 pair 的资源。CLI 报告资源遗漏。
原片保持完整。产品 pair 转换不承诺完整 vendor 资源保留或无损。
需要这种保留时，使用独立库的 source-copy 或显式 sidecar 策略。

验证使用现有 parser/payload conformance 和 14 个固定原片的 CLI 转换门禁。
runtime 测试另行比较源时间与写入时间，并检查转换报告。
completion gate 将本地检查绑定到已提交 HEAD；GitHub 提供第二验证环境。

原厂 Gallery/Photos 验收是独立要求。read、write、fixture、CI 和 device 状态
不得合并成一个支持结论。完整支持与证据边界见英文 canonical 文档的链接。

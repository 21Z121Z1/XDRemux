# 真实 Motion Photo Fixture

[English](README.en.md) | 简体中文

本目录保存 strict Swift 和纯 Python CI gate 使用的真实 Motion Photo 输入。

## 文件身份契约

媒体文件按原始字节作为测试 fixture 提交。

`SHA256SUMS` 是 canonical identity manifest。

strict fixture test 必须拒绝字节与记录 digest 不一致的文件。

如果测试依赖原始容器布局，不要为了“规范化 metadata”而改写 fixture。

## Fixture 保留的数据

真实 fixture 可能包含：

- EXIF；
- 拍摄时间；
- 厂商 metadata；
- 内嵌 Motion Video 资源；
- Gain Map；
- 方向数据；
- parser 或 validator 所需的其他源 payload。

这些资源属于测试输入的一部分。

不要因为文件位于测试目录，就假设真实照片已经去敏。

## 覆盖范围

当前 corpus 包含多种 Android Motion Photo 实现，以及 JPEG 和 HEIC/HEIF 布局。

公开文档不把这个 corpus 当成厂商 allow-list。一个 fixture 只能证明对应文件结构和测试场景的行为。

## 生成输出

生成的 Live Photo HEIC/MOV 是临时测试或 workflow artifact。

除非未来测试明确把某个输出定义为版本化 golden artifact，否则不要把生成转换结果提交到本目录。

## 增加 fixture

增加真实 fixture 时：

1. 确认该文件可以公开提交。
2. 不修改字节，提交原始文件。
3. 在 `SHA256SUMS` 中加入 SHA-256 digest。
4. 增加或更新测试，明确该 fixture 能证明什么。
5. 不要根据一个 fixture 推断无关的设备支持。

仓库测试政策见 [docs/quality/testing.md](../docs/quality/testing.md)。

# 厂商 Live Photo 几何证据摘要

[English](vendor-live-photo-geometry.summary.en.md) | 简体中文

这是证据记录 [vendor-live-photo-geometry.md](vendor-live-photo-geometry.md) 的当前语言摘要。

原文件记录厂商特有 Live Photo 几何工作的证据边界。保留其中的详细观察作为证据。

## 当前稳定结论

production Live Photo 链路保留配套 Motion Video bitstream，不把几何校正直接渲染到视频像素。

对于 vendor geometry policy 覆盖的输入，转换器可以使用源 metadata 和只用于分析的辅助资源，选择 Live Photo transform metadata。

几何分析失败不一定导致整个 Live Photo 转换失败。实现允许时，production 链路可以使用受支持的 metadata fallback。

## 证据边界

不能因为某个 Apple 私有逐帧 payload 或其他私有 metadata 的二进制形状看起来合理，就直接写入。

当前可以写哪些几何 metadata，以当前代码和测试为准。原记录用于解释为什么部分私有 payload 被有意排除。

# 编码质量审计摘要 — 2026-07-18

[English](encoding-quality-pareto-20260718.summary.en.md) | 简体中文

这是历史记录 [encoding-quality-pareto-20260718.md](encoding-quality-pareto-20260718.md) 的当前语言摘要。

原文件属于证据记录。不要为了匹配后续代码而改写其中的测量值。

## 记录测量了什么

该审计比较当时 XDRemux 主动编码 payload 的质量、体积和编码行为。

其中包含：

- portrait `src.image` base 编码；
- Gain Map HEVC 编码；
- Gain Map tile size；
- 摄影风格 Linear Thumbnail 编码；
- 原文件记录的其他 payload 决策。

## 如何使用

需要 2026-07-18 的具体测量值时，直接使用原文件表格。

不要在没有检查当前 encoding policy 的情况下，假设旧的选定 quality 仍然是当前代码默认值。

当前产品行为以当前代码和当前技术文档为准。

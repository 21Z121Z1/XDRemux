# Palette 准入实验

[English](README.en.md) | 简体中文

这些实验测试 ColorOS 相册是否接受两种候选 Palette 表达。**相册端的结果仍未验证。** 实验工具不会为 XDRemux 增加 Palette 转换模式。

容器检查通过，只能证明检查器可以读取该候选文件。它不能证明相册接受文件、显示 Palette 控件或保留可逆编辑能力。

## 实验定义

[`probe.py`](probe.py) 保留了 PR #44 和 #45 的研究问题。它使用独立的解析器，不恢复已停用的 Python 产品解析器。

| 命令 | 修改内容 | 证据边界 |
| --- | --- | --- |
| `context` | 将源文件已有的 `basictone.info`、`basictone.lmtlut.table`、`basictone.vig.table` 和 `hdr.transform.data` 复制到独立的 HEIC 基础文件。 | 基础文件必须来自同一源图。工具不会通过解码像素来验证图像身份。 |
| `filter-info` | 将清单中的 `filter.info` 引用改为新的 220 字节小端 `FilterPhotoInfoV51` 载荷。 | 写入值都是合成实验输入，不是对源图的测量结果。 |

`context` 命令排除滤镜、水印、预设、人像和 Motion Photo 条目，以便单独比较渲染上下文。这个实验不是无损照片转换。

公开样本 [`coloros16-dualstream-ultrahdr-01.jpg`](../../fixtures/motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg) 在上述允许列表中仅包含 `hdr.transform.data`。不能据此声称样本含有 BasicTone 表，也不能声称相册使用了这些表。

`filter-info` 命令保留了历史候选标识符 `palette-default` 和 `default`。合成值如下：

| 字段 | 候选值 |
| --- | --- |
| 版本 | 5.1 |
| 强度 | 100 |
| Filter/Palette 标志 | 按所选假设设置 |
| 色温 | 6500 |
| 照度 | 100 |
| 饱和度和色调 | 0 |
| 亮度值 | 0.18 / 0.75 / 0.03 |
| 拍摄模式 | 由命令参数明确指定；下方示例为 `common` |

## 数据保留规则与解析限制

滤镜实验保留原清单之前的全部字节，包括未列入清单的间隙和被替换的旧滤镜载荷。工具调整反向偏移，保留未知字段，并保持未修改条目的顺序。

解析器拒绝重复条目名称、重复 JSON 键、重叠区间、无效整数类型和越界尾部。布尔值不能用作此格式的整数偏移。解析器仅支持已观察到的尾部布局：JSON、NUL、四字节标签和小端长度。其他布局会明确失败。

## 运行实验

从仓库根目录运行。先安装 Python 3；使用检查命令前还需构建 Rust CLI。输出路径必须尚不存在，工具拒绝覆盖已有文件或符号链接。

```sh
python3 research/palette/probe.py filter-info \
  fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic \
  candidate.heic --filter-type palette-default --capture-mode common

target/debug/xdremux inspect candidate.heic --json
python3 -m unittest tests.test_palette_research
```

在 macOS 上运行 `context` 实验：

1. 用 `sips` 将源 JPEG 转换为独立的 HEIC 基础文件。
2. 运行 `probe.py context SOURCE OUTPUT --base-heic BASE`，指定该 JPEG 及其 HEIC 基础文件。
3. 将 JSON 报告与对应的设备观察结果一起保存。

不要使用另一张图的渲染上下文。公开源样本的身份记录在 [`fixtures/SHA256SUMS`](../../fixtures/SHA256SUMS) 中。

报告采用 `xdremux-palette-experiment-v1` 模式，记录源文件、基础文件和输出文件的哈希、保留条目及所选假设。报告还明确记录 `research_only: true` 和 `consumer_verdict: unverified`。

工具先在目标目录内写完临时文件，再用原子硬链接发布。发布时拒绝覆盖已有目标。不支持硬链接的文件系统会失败，不会改用覆盖方式。标准库行为见 [`os.link`](https://docs.python.org/3/library/os.html#os.link) 和 [`tempfile.mkstemp`](https://docs.python.org/3/library/tempfile.html#tempfile.mkstemp)。

## 进入产品前的要求

对每个候选，记录准确的候选哈希、源文件哈希、设备、ColorOS 版本和相册版本。分别验证导入、控件显示、编辑、保存、重新打开和持续可逆性。

保留负结果。不要仅为显示某个控件而改动无关元数据。进入产品的前提是格式约定忠实于源数据，有明确的相册端行为证据，并通过 Rust 主路径的真实样本回归。

旧的一次性工作流及其过时解析器依赖已停用。此实验不下载固件、不上传私有媒体，也不自动宣称设备验证成功。

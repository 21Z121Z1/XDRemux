# Palette 准入实验

[English](README.en.md) | 简体中文

状态：**研究；消费者结论未验证**。这些工具不会给 Rust 产品增加 Palette 模式。
容器能被 inspector 读取，不等于 ColorOS 相册认可它或支持可逆编辑。

## 两个假设，一个工具

`probe.py` 重建 PR #44 和 #45 的有效实验意图，不恢复已删除的 Python 产品解析器。

- `context` 只把同一源图已有的 `basictone.info`、`basictone.lmtlut.table`、
  `basictone.vig.table` 和 `hdr.transform.data` 放到从该源图解码得到的 HEIC
  base 上。不会携带 filter、水印、preset、人像和 Motion Photo 项。这是消融实验，
  不是无损照片转换。原来的公开 ColorOS 16 样本现在位于
  `fixtures/motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg`，在上述白名单中
  仅包含 `hdr.transform.data`。不能据此声称样本包含 BasicTone 表或相册消费了这些表。
- `filter-info` 只把 manifest 中的 `filter.info` 引用改为新的 220 字节小端
  `FilterPhotoInfoV51` 假设。保留 `palette-default` 和 `default` 两种历史候选。
  版本 5.1、强度 100、filter/Palette 标志、色温 6500、照度 100、饱和度和 tone
  为零、亮度值 0.18/0.75/0.03、显式 capture mode 都是**合成的实验输入**，不是
  对源照片的测量结果。

filter 实验保留原 manifest 之前的每一个字节，包括未列出的间隙和被替换的旧
filter payload。仅调整反向偏移，保留未知字段以及未修改记录的次序，不重新打包
无关数据。解析器拒绝重复名称、重复 JSON key、重叠区间、非整数偏移和越界 footer。
仅支持已经观察到的尾部 JSON/NUL/四字节 tag/小端 span 布局；其它布局明确失败。

## 复现

从仓库根目录运行。输出路径必须是新的，不覆盖已有文件或符号链接。

```sh
python3 research/palette/probe.py filter-info \
  fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic \
  candidate.heic --filter-type palette-default --capture-mode common

target/debug/xdremux inspect candidate.heic --json
python3 -m unittest Tests.test_palette_research
```

macOS 上先用 `sips` 将同一 JPEG 单独转成 HEIC base，然后运行
`probe.py context SOURCE OUTPUT --base-heic BASE`。样本身份记录在
`fixtures/SHA256SUMS`。不要使用另一张图的 render context。程序以 JSON 输出源图和
base 的哈希、输出哈希、保留条目以及假设，应与设备观察一同保存。

发布使用同目录内已经写完的临时文件和原子、不覆盖的硬链接。不支持硬链接的
文件系统会明确失败，不降级为覆盖。参见 Python 标准库的
[`os.link`](https://docs.python.org/3/library/os.html#os.link) 和
[`tempfile.mkstemp`](https://docs.python.org/3/library/tempfile.html#tempfile.mkstemp)。

## 晋升门槛

记录候选与源图的精确哈希、设备、ColorOS 版本和相册版本。对每个候选分别验证导入、
控件显示、编辑、保存、重开以及持续可逆性。负结果也是有效证据；应记录它，而不是
不断改动无关元数据直到控件出现。只有消费者证据支持、忠实于源数据的最小契约，
并通过 canonical Rust 路径的真实样本回归，才能进入产品。

旧的一次性 workflow 以及过时解析器依赖已退役。实验不下载固件、不上传私有媒体，
也不自动声称设备验证成功。

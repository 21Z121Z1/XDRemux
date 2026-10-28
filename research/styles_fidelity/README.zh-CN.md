# 摄影风格真实性实验

[English](README.md)

本研究为盲重建对照准备输入：用同一张照片的 SDR／HDR 信息重建资源，再与原始 Apple Styles 资源比较。本次不改变产品生成流程、标定常数或支持的输入类型。

## 准备样本

```sh
python3 -m research.styles_fidelity.corpus \
  --output /absolute/path/new-corpus /absolute/path/photos.zip
```

目录记录压缩包与成员身份、文件级重复、压缩主图指纹、关联的 Styles 资源和有类型的 SmartStyle 状态。原始 HEIF 使用哈希文件名保存；压缩包内路径不会直接展开。脚本不执行包内代码，也不提取系统二进制。

Standard 候选需要 Apple EXIF、已确定的中性 SmartStyle 字段及相关资源，并且不能具有非 Standard 纹理预设。这只是校准样本筛选规则。元数据和压缩包来源均不能认证拍摄来源；缺失编辑状态保持未知，纹理预设本身不足以判断。

参考目录会标出超出当前产品 `[4, 10.4]` 标定范围的 baseline。这项诊断不能证明渲染错误，也不直接支持改变范围；需要原生渲染确定影响。

## 隔离生成器输入

```sh
python3 -m research.styles_fidelity.isolate \
  --input /absolute/path/original.heic --output /absolute/path/new-case
```

按允许清单投影资源：保留压缩主图、一张关联的 Apple HDR Gain Map、编码依赖、已核对的像素属性、存在时的匹配色调映射项目及数值 HDR 描述符。投影重新建立项目图和媒体存储，原 Styles、Linear Thumbnail、Delta Map、语义资源、SmartStyle MakerNote、无关 XMP 和未使用的媒体字节不会进入生成器文件。EXIF 从零重建，仅写入固定的 Apple 品牌名和 HDR MakerNote 数值字段 33／48，保留其有理数值；其他 EXIF 字段与 MakerNote 状态全部去除。删除 HDR 字段可能在 SDR 像素不变的同时禁用 Apple HDR 展开，因此必须进行原生检查。写出后再次核对像素载荷、颜色和几何。属性不支持或关联存在歧义时失败。

拟合前先审阅场景分组。分组 JSON 须含 `scene_groups_reviewed: true` 及按候选 SHA 索引的 `samples` 字典；每条提供 `scene_group` 和 `split`（`calibration` 或 `holdout`）。

```sh
python3 -m research.styles_fidelity.split \
  --corpus /absolute/path/new-corpus/corpus.json \
  --groups /absolute/path/reviewed-groups.json --output /absolute/path/new-split
```

同一场景和同一压缩主图不能跨组。生成器清单只有隔离输入；单独的评估清单保存原件与标签。这份划分仅适用于本实验，不证明旧研究没有使用过照片。原生等价性在实测前保持未验证。

## 核对原生等价性

在受支持的 arm64／macOS 主机构建现有 Apple 适配器，再执行：

```sh
python3 -m research.styles_fidelity.native \
  --adapter /absolute/path/xdremux-apple-adapter \
  --input /absolute/path/original.heic --output /absolute/path/new-native-result
```

检查比较原件与隔离文件完整、按显示方向排列的 RGBA32F 线性 Display P3 SDR／HDR 回读。要求源文件不变、headroom 不变、像素有限，最大绝对误差不超过 `1e-6`，且参考文件确实展开 HDR。比较后删除临时像素缓冲；报告只有数值、尺寸和哈希。已有输出不会复用。目前明确拒绝 clean aperture 几何。

现有 `research.yml` 的 Tone 任务对固定版本的公开 Apple 样本运行此检查，仅上传报告。用户提供的私人照片保留在 Git 和公开产物之外。便携解析或解码不能证明 Apple HDR 等价、重建后的滑块响应或 Photos 编辑／保存／重开。

## 后续实验

1. 每份私人样本通过原生 SDR／HDR 等价检查后，才能进入下一阶段。
2. 对隔离输入调用共享的 Rust 资源生成器；原系数、曲线、蒙版和统计仅供评估使用。
3. 改变曝光、GTC 或局部拟合标定前，对照原生非零 Tone／Color 及移除风格结果；拟合前固定场景划分。
4. 将已有纹理最终文件重放用于完整 HDR 产品输出，记录解码蒙版、颜色域、尺寸和逐效果响应。
5. 同帧人物提亮曲线捕获、RAW 验证与元数据合成分开；逆问题不唯一时保留不确定性。

便携边界回归：`python3 -m unittest tests.test_style_fidelity -v`。

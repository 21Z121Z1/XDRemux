# 摄影风格真实性实验

[English](README.md)

本研究用同一张照片隔离后的 SDR／HDR 信息重建资源，再与原始 Apple Styles 的原生编辑响应比较；同时使用最终 HDR 产品文件中解码出的蒙版、人物数据执行原生纹理效果重放。产品默认参数和支持的输入类型保持不变。

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

按允许清单投影资源：保留压缩主图、一张关联的 Apple HDR Gain Map、编码依赖、已核对的像素属性、存在时的匹配色调映射项目及数值 HDR 描述符。投影重新建立项目图和媒体存储，原 Styles、Linear Thumbnail、Delta Map、语义资源、SmartStyle MakerNote、无关 XMP 和未使用的媒体字节不会进入生成器文件。EXIF 从零重建，仅写入在源文件中核实的 Apple 品牌名和 HDR MakerNote 数值字段 33／48，保留其有理数值；其他 EXIF 字段与 MakerNote 状态全部去除。删除 HDR 字段可能在 SDR 像素不变的同时禁用 Apple HDR 展开，因此必须进行原生检查。写出后再次核对像素载荷、颜色和几何。属性不支持或关联存在歧义时失败。

同时保留匹配 tone-map 项目的有序替代图像组、经允许清单核对的源 HDR 格式声明、项目可见性与源 `idat`／`mdat` 构造方式；紧凑项目 ID 使用 16 位 HEIF 布局。匹配的 tone-map 项目不能被改成隐藏状态。

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

加上 `--ablate-hdr-exif` 可额外对照不含数值 HDR EXIF 的更严格投影，记录该文件的 HDR 表示是否依赖这些字段。这个对照不会豁免默认投影的失败。

现有 `research.yml` 的 Tone 任务对固定版本的公开 Apple 样本运行此检查，仅上传报告。用户提供的私人照片保留在 Git 和公开产物之外。便携解析或解码不能证明 Apple HDR 等价、重建后的滑块响应或 Photos 编辑／保存／重开。

## 重建资源与联合校准

`styles-research` Cargo 特性提供研究示例，对隔离后的 Apple HDR 基图调用共享 Rust Styles 生成器；它不把源文件伪装成 ProXDR，也不扩展产品 CLI 准入。引擎接受明确的 baseline、gain 缩放和 baseline 上限，默认值沿用现有产品标定。每次试验共同重建缩略图、GTC、系数、光照图、统计和元数据；原资源仅供评估。

```sh
cargo build --locked -p xdremux-runtime --features styles-research --example styles_fidelity
python3 -m research.styles_fidelity.blind \
  --adapter /absolute/path/xdremux-apple-adapter \
  --generator /absolute/path/target/debug/examples/styles_fidelity \
  --research-root /absolute/path/pinned-consumer-research \
  --calibration /absolute/path/calibration-a.heic \
  --calibration /absolute/path/calibration-b.heic \
  --holdout /absolute/path/holdout.heic --output /absolute/path/new-fit
```

每例先通过完整原生 SDR／HDR 输入等价性检查，评估响应前固定场景划分。默认试验与 3×3 曝光／增益网格仅按校准组原生编辑 RMSE 选择；选择完成后才评估留出组。报告记录非零 Tone、移除风格响应；只有原生滤镜确实提供 Color 控制时才执行 Color 响应。缺失能力明确保留为未测。数值比较覆盖暗部、中间调和高光；替代拟合不能豁免默认试验的失败。

CI 使用公开的三份语义 Standard 对照 `IMG_2600/2604/2608`。其 Studio 纹理元数据没有人物，因而不能激活人物效果。这些是旧研究已使用过的公开风景对照，不能替代更严格的私人 Standard 样本集，也不支持人像或夜景泛化结论。已有私人样本的筛选与场景划分保持不变；其原生拟合需要私人 Mac 环境，照片不会发送到公开 Actions 或公开产物。

## 重放最终 HDR 文件的纹理资源

```sh
python3 -m research.styles_fidelity.texture \
  --adapter /absolute/path/xdremux-apple-adapter \
  --research-root /absolute/path/pinned-texture-research \
  --input /absolute/path/current-product.heic --output /absolute/path/new-replay
```

固定版本的纹理原语来自 `texture.py` 指定的研究提交。源文件必须包含原生人物，并实际产生高于 SDR 白点的像素。蒙版由最终编码资源投影后交给 ImageIO 解码，人物几何与统计来自最终文件；重放不使用新 Vision 观察或编码前蒙版。尺寸须与产品的最长边 1024 工作域吻合，不提供蒙版缩放回退。

对每个人物分别渲染 Mattify、UnderEyeBrighten、SkinSmoothStandalone，测量产品 SDR 派生线性 P3 分析图和按面积缩小的原生线性 P3 HDR 两种受控域。后者绕过 8 位 SDR 中间图，保留大于 1 的值。每个域分别使用原统计，以及清除统计但保留最终文件人物几何后从最终解码蒙版重新计算的统计进行渲染，两者像素差异用于测量编码前统计的敏感性。报告保留 GPU 状态、有限像素检查、输出变化及原统计与重算统计的差异；跨域差异表示域敏感性，不是相对 Photos 参考渲染的误差。这里尚未验证全尺寸效果合成、多人物混合或设备保存重开。

现有研究 Tone 任务先由当前产品 CLI 转换 `uhdr-portrait-02.heic`，再执行两项实验，仅导出数值报告。这份公开样本拍摄的是商店人体模特，不能验证真人皮肤真实性；其原生检测的人物上下文用于核查资源重放。`uhdr-portrait-01.heic` 拍摄的是玩偶，没有原生人脸上下文。图像、人物关键点、输入资源与像素缓冲留在本次临时目录中。

## 后续实验

1. 每份私人样本通过原生 SDR／HDR 等价检查后，才能进入下一阶段。
2. 标定只有通过更广的独立场景和原生编辑响应验证后才能进入产品；此前保留现有默认值。
3. 将纹理重放与完整 Photos 合成及真机编辑比较，覆盖多人物交互和全分辨率输出。
4. 同帧人物提亮曲线捕获、RAW 验证与元数据合成分开；逆问题不唯一时保留不确定性。

便携边界回归：`python3 -m unittest tests.test_style_fidelity -v`。

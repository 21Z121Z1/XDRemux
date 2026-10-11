# 待预处理的 Cinematic MOV

[English](cinematic.en.md) | 简体中文

`xdremux cinematic` 从普通视频生成新的待预处理 Cinematic MOV。验收目标是 Apple 公共 API 的 `needsPreprocessing` 状态。命令不保证推理出的电影效果与原始相机效果相同。

## 命令与运行条件

```bash
xdremux cinematic --input ordinary.mp4 --output pending.mov
```

输出文件必须尚不存在，输入文件保持不变。Rust runtime 在生成前后计算输入哈希，通过不会替换现有输出的硬链接发布候选文件。

当前实现需要 macOS 27、可用的 Apple 推理资源，以及 adapter 使用的 Portrait/Cinematic 私有框架方法。adapter 在调用前检查每个必要 Objective-C 方法的类型编码。方法或资源不受支持时，命令会失败。

`--focal-length-35mm MM` 设置每帧使用的焦距假设，默认为 24，允许范围是 1 到 300。`--aperture F` 默认为 2.8，允许范围是 1 到 64。两个值都必须有限。缺少相机几何信息时，它们是生成元数据使用的拍摄假设，不是实测光学属性。

## 原生生成路径

1. 使用 AVFoundation 解码选定的彩色视频轨道。
2. 使用 Apple 的 `PTMonocularDisparityProvider`，按假设焦距从解码帧推理临时视差。使用 Vision 检测人脸，没有人脸时检测上半身。将视频显示方向传给 Vision，再把检测矩形映射回编码坐标。优先选择较大的主体，并在可行时延续附近的上一帧锚点。两个检测器都没有找到主体时，选择置信度最高的对象显著性区域。置信度必须至少为 0.3，区域面积必须占画面的 0.5% 到 60%。没有合适的显著性区域时，使用画面中心区域。对象和中心区域使用合成的脚本锚点，不表示检测到了人物。
3. 使用 Apple 的轨道/对焦标识分配器和 `PTCinematographyPostcaptureRefinement` 生成并序列化连贯的逐帧脚本。
4. 写入新的全局 Cinematic 元数据，并为每个解码帧写入一组逐帧 Cinematic 元数据。未知的拍摄对焦/视差使用原生待处理标记 `-1`。可见裁切设为 `(1,1)`，使 Apple 的解析器保留逐帧假设焦距。虚拟传感器区域设为编码图像尺寸。脚本和逐帧渲染数据都写入光圈。
5. 将压缩视频及全部受支持的音轨复制到 MOV，保留视频变换和轨道时间基。元数据生成后移除临时推理缓冲，不写入栅格视差轨道。
6. 使用公共 `CNAssetInfo` 加载新 MOV。只有确认 `needsPreprocessing`、资源状态 `ready` 和 `isPreprocessed == false` 后，runtime 才发布文件。

全局渲染数据使用兼容的虚拟拍摄配置和渲染版本 5。硬件/传感器字段、虚拟传感器区域及由亮度推导的场景照度代理值都是合成值，不能恢复原相机标定、防抖裁切或原始对焦选择。输出不会复制任何原生样本的 Cinematic 载荷或视差轨道。

生成器保持输入的焦距假设不变，不从图像运动推理光学变焦或绝对视场角。原始 Cinematic 几何、视差图和对焦决策仅作为诊断参考。

Rust CLI 仍是产品入口。Swift/Objective-C adapter 执行 Apple 框架操作。输出是独立文件。

## 验证

私有样本可以保留在 Git 外。在本工作树中执行以下命令，使用新的结果目录和新的或已有的构建目录。结果目录必须尚不存在。

```bash
python3 scripts/verify_cinematic_envelope.py \
  --input /absolute/path/native.MOV \
  --run-dir /absolute/path/new-results \
  --swift-build-dir /absolute/path/swift-build \
  --rust-build-dir /absolute/path/rust-build \
  --synthetic --compare-render
```

重复 `--input` 可加入更多原生样本。runner 构建当前 CLI 和 adapter，移除各原生样本的全局元数据以及所有非视频/非音频轨道。公共 `CNAssetInfo` 必须将普通副本判定为 `none`。随后 runner 调用 CLI，生成独立的待处理元数据封装。

公共消费者检查会验证脚本、选定检测对象 ID、帧数、时间戳、光圈、逐帧键、渲染帧属性、虚拟传感器几何、逐帧焦距范围以及待处理资源状态，并记录焦距时间线。FFprobe 比较实际编码视频/音频包的哈希、时间戳和音频裁切数据。AVAssetReader 的音频缓冲分组不能用于证明媒体包相同。runner 在各操作后复查源文件哈希。`--synthetic` 还会加入没有原始 Cinematic 数据及可检测人脸的 H.264 输入。

## 无头电影效果对照

`--compare-render` 分别调用公共 `CNAssetInfo` 预处理原生资产和重新生成的资产，两者都必须变成 `renderable`。随后使用公共 `CNRenderingSession` 的 export 质量，在共同时间范围的 25%、50% 和 75% 位置分别以 f/2 和 f/16 渲染。两者使用相同解码色彩、时间戳、10 位输入格式、输出几何和单位显示变换。

测量使用完整分辨率的 `RGBA16Float` 纹理 RGB 数值。原生 RGBA 输出保留源传递函数。HLG 输入对应 HLG 编码的 BT.2020 RGB，并非线性 sRGB。工具还将渲染图像转换为扩展线性 BT.2020，单独记录线性渲染与效果差异。两类测量都不包含 alpha。工具计算解码色彩平面的哈希，并要求输入完全相同。报告记录像素完全相同与否、平均绝对误差、均方根误差、峰值信噪比，以及绝对差大于 0.001 的分量比例，同时记录各对焦矩形。工具比较推理视差及 f/2 与 f/16 的效果，分别测量完整画面和原生对焦矩形。效果余弦相似度用于判断虚化变化是否发生在相近区域。重复渲染原生资产用于测量确定性。

将生成版本设为原生对焦数值的渲染仅用作诊断对照。视差的缩放和偏移没有定义，相同的数值不能证明两份不同视差图采用相同的物理对焦平面。

比较新旧两轮验证结果时，使用：

```bash
python3 scripts/compare_cinematic_alignment.py \
  --baseline /absolute/path/baseline/report.json \
  --current /absolute/path/current/report.json \
  --output /absolute/path/current/alignment.json \
  --require-improvement SAMPLE_NAME --minimum-reduction 0.25
```

工具要求当前报告对应 `HEAD`，检查源文件哈希、普通输入完整性、媒体包一致性、解码色彩一致性、采样时间、渲染设置、原生渲染可重复性，以及参考效果能量是否不变。指定样本的平均 f/2 渲染误差和平均相对效果误差都必须下降。报告也会列出其他全部样本，包括退步的样本。该检查不会把测得的改进变成效果等同保证。

此前的报告将未经转换的纹理数值错误标注为 `raw linear RGB`。只有解码色彩哈希相同且原生参考效果能量不变时，才允许把这些编码 RGB 测量与当前的编码 RGB 测量比较。比较工具会记录标注修正。此前将 HLG 纹理解释为线性 sRGB 的 PNG 预览不能作为有效视觉参考。

PNG 是 SDR 预览。Core Image 使用解码缓冲的色彩空间、内容亮度余量和平均亮度。`CISystemToneMap` 以显示亮度余量 1 映射到标准动态范围。工具将源视频的显示变换从视频坐标转换到 Core Image 坐标，写入含 sRGB 配置文件的 8 位 PNG。这些预览不是浮点测量数据，也不证明与 Photos 的 Dolby Vision 动态修剪播放相同。测试预处理副本引用不变的输入彩色轨道，以避免重复存储，因此这些测试副本不能独立搬移。产品 `pending.mov` 仍是独立文件。

使用验证 runner 构建的渲染器运行 HDR 色彩回归：

```bash
/absolute/path/new-results/cinematic-render-comparison \
  --verify-color /absolute/path/new-color-results
```

已知 HLG 色块必须与经过色彩管理的参考一致。此前的线性 sRGB 解释必须无法通过负对照。SDR PNG 必须包含 sRGB 配置文件，并在系统色调映射后保留可区分的高光层次。不对称角标在 0、90、180、270 度下必须与独立的 Core Image 方向参考一致。

对照执行完成不代表效果相同。应结合 `all_compared_frames_identical` 和数值差异评估一致性。缺失的相机标定、光学几何或原始对焦决策，会让能够正常渲染的两份资产产生差异。本测试不证明 Apple Photos 保存/重开行为、设备相册行为，或未抽样帧的相同性。

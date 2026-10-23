# GeoCalib 模型适配器

[English](README.en.md) | 简体中文

这个本地模型原语对固定视角的 SDR 视频抽帧运行 GeoCalib。当 Cinematic 输入没有固定的已记录 35mm 等效焦距时，Rust runtime 自动调用它。它独立估计每帧，拟合共享相机，返回绑定源视频的像素内参和不确定度。Rust engine 准入观测并选择一个恒定的有效等效焦距。Apple adapter 将该值用于临时视差推理和每帧渲染元数据。模型 helper 不选择转换 policy，也不生成 MOV。

模型不能恢复物理镜头标定、拍摄光圈、对焦距离、Apple 对焦视差或防抖裁切，也不向 Photos 提供深度图。单图相机估计即使不确定度较低也可能出错，抽帧检查不能证明未采样帧的相机几何不变。首版要求先将剪辑或变焦视频分成固定视角片段。

## 安装和使用

在 `PATH` 中安装用于自动读取元数据的 ExifTool，以及用于模型解码的 FFmpeg 和 FFprobe。使用独立 Python 环境。已测试路径是在 Apple silicon 上以 CPU 运行 Python 3.13、PyTorch 2.14.1 和 torchvision 0.29.1。从仓库根目录运行：

```bash
python3.13 -m venv /path/to/camera-env
/path/to/camera-env/bin/python -m pip install -r model_adapters/geocalib/requirements.txt
curl --fail --location https://github.com/cvg/GeoCalib/releases/download/v1.0/geocalib-pinhole.tar --output /path/to/geocalib-pinhole.tar
python3 scripts/distribution/install_geocalib.py \
  --python /path/to/camera-env/bin/python \
  --weights /path/to/geocalib-pinhole.tar --bin-dir /path/to/xdremux/bin
/path/to/xdremux/bin/xdremux convert --input fixed-view.mov --output pending.mov --apple-cinematic
/path/to/xdremux/bin/xdremux batch --input-dir videos --output-dir pending --apple-cinematic
```

将示例路径替换成持久位置。`--bin-dir` 必须存在，通常包含 `xdremux` 和 `xdremux-apple-adapter`。安装器检查固定权重哈希，发布自包含的 `xdremux-geocalib` launcher，不覆盖已有 helper。launcher 内嵌模型原语，以绝对路径引用所选 Python 环境和权重，安装后不依赖源 checkout。需保留对应环境和权重。产品压缩包不包含这些依赖。转换不会自动安装 package 或下载权重。

Rust 从 Apple adapter 同目录查找 ExifTool 和 GeoCalib，其次查找 `PATH`。`XDREMUX_EXIFTOOL` 和 `XDREMUX_GEOCALIB` 可覆盖可执行文件路径。显式 `--focal-length-35mm` 绕过这两个工具。自动元数据准入只采用主资源 EXIF/XMP `FocalLengthIn35mmFormat` 或 QuickTime `camera.focal_length.35mm_equivalent`，排除物理焦距、合成字段和缩略图。零值表示未知。已知值格式错误或冲突时失败；缺失或未知才调用 GeoCalib。两条路径都不会静默代入 24 mm。

模型本地运行，不上传媒体。默认均匀选取五个采样目标时间。按显示方向解码第一条视频轨道，不裁切地将完整画面缩放到每边至多 1280 像素，记录该坐标下的内参。拒绝 HDR 传递函数、非正方形源像素和封面图轨道。Rust 在调用原生 adapter 前验证生成的 JSON，操作结束时删除私有的逐转换 JSON。模型失败、依赖缺失或观测被拒绝时失败，不发布 MOV。

helper 要求 GeoCalib 源码版本 `1dedb29f94fc11b896c5f572005978aeebccd67f`，并检查已安装包的来源。发布权重 SHA-256 必须为 `86d6aeacd8bbd974c59ce39f61854e00d36911c732ad89be471476fd708722ac`。推理前后都重新计算输入和权重哈希。Rust 将估计源哈希与发布 MOV 前重新核对的同一源哈希比较。

需要可复现的诊断观测时，仍可直接运行原语：

```bash
/path/to/camera-env/bin/python model_adapters/geocalib/estimate.py \
  --input fixed-view.mov --weights /path/to/geocalib-pinhole.tar --output camera.json
xdremux convert --input fixed-view.mov --output pending.mov \
  --apple-cinematic --cinematic-camera-estimate camera.json
```

直接诊断时 `--samples` 接受 3 至 128。已有 JSON 不会被覆盖。`--cinematic-camera-estimate` 仅供 `convert` 使用，要求 `--apple-cinematic`，高于自动元数据/模型选择。显式 `--focal-length-35mm` 优先，且不读取 JSON。光圈仍由独立的 `--aperture` 设置，默认 f/2.8。

已知源相机几何或明确的拍摄先验应优先于单图估计。若防抖进行均匀裁切，裁切系数为 c，则有效视角等效焦距约为标称等效焦距乘以 c。裁切会缩小视场并增大这个等效值，不能把已知的 70 mm 视角解释为 40 mm。仅凭标称焦距无法恢复未知的 EIS 裁切。

## 观测约定

版本 1 包含 `source_sha256`、具有 `name`、`revision`、`weights_sha256` 的 `estimator` 对象和 `samples`。每个样本包含采样目标 `time_seconds` 和 `intrinsics` 对象。可选的 `shared_intrinsics` 对象记录固定视角的联合拟合。每个内参对象包含 `image_size` 的 `[width, height]`、`focal_length_pixels` 的 `[fx, fy]`、`principal_point_pixels` 的 `[cx, cy]`，以及 `focal_uncertainty_pixels`。主点单位是像素；GeoCalib 将其假设为图像中心，并未独立恢复主点。不确定度是 GeoCalib 在采样图像像素单位下的焦距不确定度。其他估计器必须提供相同单位与语义，不能用置信分数替代。

Rust 拒绝超过 1 MiB 的 JSON、不受支持的 schema、来源缺失、源哈希不符或不在 3 至 128 范围的样本数。采样时间必须有限、非负并严格递增。图像每边须为 1 至 16384。焦距必须为正且有限，`fx/fy` 与 1 的偏差不得超过 1%。主点与图像中心的偏差不得超过一个像素。全部不确定度必须有限且非负。选中的共享拟合不确定度不得超过 `fy` 的 20%；没有共享拟合时，每个独立样本都必须满足该上限。这些是保守准入检查，不是经过标定的精度保证。

Rust 对每个合格样本计算：

```text
equivalent_mm = hypot(36, 24) / hypot(width / fx, height / fy)
```

该公式将对角视场匹配至虚拟 36 × 24 mm 画幅，均匀缩放和旋转 90 度不改变结果。它是有效视角的估计，不是源相机的物理焦距。每个结果必须在原生接口支持的 1 至 300 mm 内。有共享拟合时 Rust 选择该拟合，否则选择独立样本的中位数。每个独立估计焦距与选中值的偏差都不得超过 20%。共享拟合不能掩盖逐帧估计冲突；共享拟合失败时也不会退回独立样本。它在调用 Apple adapter 前拒绝不确定或不一致的观测，估计失败时不会静默退回 24 mm。

固定 GeoCalib 版本的共享模式不确定度访问器使用协方差索引 2，该位置在多帧拟合中对应第二帧的重力参数。联合针孔系统先排列 `2 * sample_count` 个重力参数，最后排列单一焦距参数。工具读取最后的焦距方差，沿用 GeoCalib 的平方根再除以 2 的约定，然后还原垂直预处理缩放。工具检查协方差维度，不把上游取错重力参数得出的数值当作焦距确定性。PyTorch 随机种子固定为零，用于模型内部随机基的生成。

准入使用的源哈希与 runtime 在发布前复查的是同一个哈希。Runtime 还要求 adapter 报告的焦距和光圈符合请求。已有输出保护和原生待处理状态校验仍属于转换流程。模型执行、Rust 准入、原生预处理和 Photos 编辑是不同证据层。新的估计或成功生成待处理 MOV 不能证明模糊质量改善，也不能证明物理相机参数准确。

GeoCalib 由其作者维护，见 [cvg/GeoCalib](https://github.com/cvg/GeoCalib)、[代码许可证](https://github.com/cvg/GeoCalib/blob/1dedb29f94fc11b896c5f572005978aeebccd67f/LICENSE) 和[权重许可证](https://github.com/cvg/GeoCalib/blob/1dedb29f94fc11b896c5f572005978aeebccd67f/LICENSE-WEIGHTS)。XDRemux 产品包不分发该模型。

安装后的 launcher 使用 Python 隔离模式（`-I`），保留所选虚拟环境，同时从导入搜索路径排除当前目录、`PYTHONPATH` 和用户级 site-packages。每次 FFprobe/FFmpeg 调用限时 60 秒，stdout 与 stderr 分别设限。FFprobe 只请求所需相机与时长字段，输出最多 64 KiB；解码 PNG 最多 16 MiB；诊断最多 64 KiB。超限时估计失败，并终止和回收子进程。这些传输边界不验证模型精度。

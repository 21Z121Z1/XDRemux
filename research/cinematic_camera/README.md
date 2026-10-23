# Cinematic 相机观测

[English](README.en.md) | 简体中文

这项可选研究工具对固定视角的 SDR 视频抽帧运行 GeoCalib，先逐帧独立估计，再对同组帧拟合共享相机，生成供 `xdremux convert --apple-cinematic --cinematic-camera-estimate` 使用的相机观测。Rust engine 校验观测并选择一个恒定的有效 35mm 等效焦距。现有 Apple adapter 将该值用于临时视差推理和每帧渲染元数据。

工具不能恢复物理镜头标定、拍摄光圈、对焦距离、Apple 对焦视差或防抖裁切。它不向 Photos 提供深度图。单图相机估计即使不确定度较低也可能出错，抽帧检查也不能证明未采样帧的相机几何不变。首版要求先将剪辑或变焦视频分成固定视角片段。

## 安装和使用

使用独立 Python 环境。已测试路径是在 Apple silicon 上以 CPU 运行 Python 3.13、PyTorch 2.14.1 和 torchvision 0.29.1。按以下方式安装固定版本源码与直接依赖：

```bash
python3 -m venv camera-env
camera-env/bin/python -m pip install -r research/cinematic_camera/requirements.txt
curl --fail --location https://github.com/cvg/GeoCalib/releases/download/v1.0/geocalib-pinhole.tar --output geocalib-pinhole.tar
camera-env/bin/python research/cinematic_camera/estimate.py \
  --input fixed-view.mov --weights geocalib-pinhole.tar --output camera.json
xdremux convert --input fixed-view.mov --output pending.mov \
  --apple-cinematic --cinematic-camera-estimate camera.json
```

模型和权重在本地运行，不上传媒体。估计工具需要 FFmpeg 和 FFprobe，默认均匀选取五个采样目标时间，`--samples` 接受 3 至 128。工具按显示方向解码第一条视频轨道，不裁切地将完整画面缩放到每边至多 1280 像素，记录该图像坐标下的内参。目前拒绝 HDR 传递函数、非正方形源像素和封面图轨道。JSON 输出不能覆盖已有文件。CLI 不安装或启动 Python。

工具要求 GeoCalib 源码版本 `1dedb29f94fc11b896c5f572005978aeebccd67f`，并检查已安装包的源码来源。发布权重的 SHA-256 必须为 `86d6aeacd8bbd974c59ce39f61854e00d36911c732ad89be471476fd708722ac`。观测文件记录源视频哈希、模型源码版本和权重哈希。推理后会重新计算输入与权重哈希。权重单独下载，不包含在 XDRemux 中。

`--cinematic-camera-estimate` 仅供 `convert` 使用，并要求 `--apple-cinematic`。显式 `--focal-length-35mm` 优先，此时不读取估计文件。两者都未指定时维持原有 24 mm 假设。光圈仍由独立的 `--aperture` 参数设置，默认 f/2.8。

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

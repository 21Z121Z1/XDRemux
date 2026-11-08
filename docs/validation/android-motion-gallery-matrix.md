# Android Motion Photo 相册验收矩阵

[English](android-motion-gallery-matrix.en.md) | 简体中文

本文是实机验收步骤，不代表设备已经通过验证。Rust 测试能够证明输出单文件的严格解析、视频与时间戳保留、检查点身份以及无损资源校验；不能证明相册识别、播放、HDR 显示或分享后的保留情况。官方 [Android Motion Photo 1.0 格式说明](https://developer.android.com/media/platform/motion-photo-format?hl=zh-cn)允许 JPEG、HEIC 和 AVIF，但 XDRemux 当前的 Google 产品写入器仅支持 JPEG 和 HEIC。

## 输入与输出矩阵

每组转换使用独立输出目录。先对照 `fixtures/SHA256SUMS` 核验输入 SHA-256，不要把原始照片或生成媒体上传到 CI artifacts。

| 用例 | 输入 | 输出目标 | 可移植检查 | 实机状态 |
| --- | --- | --- | --- | --- |
| J1：SDR JPEG | `fixtures/motion-photo/vivo/android-v1-sdr-01.jpg` | `google-jpeg` | 单个 `*MP.jpg`；严格解析，时间戳与视频字节一致 | 未测试 |
| J2：Ultra HDR JPEG | `fixtures/motion-photo/xiaomi/android-v1-ultrahdr-01.jpg` | `google-jpeg` | J1 加上 Gain Map 载荷与元数据保留 | 未测试 |
| H1：HEIC | 从 `samsung/heif-ultrahdr-01.heic` 的静态图像与视频通过 `batch_android_motion_photo.rs` 组装的干净 HEIC | `google-heic` | 单个 `*MP.heic`；严格解析，视频字节不变 | 未测试 |
| H2：含厂商资源的 HEIC | 未改动的 `fixtures/motion-photo/samsung/heif-ultrahdr-01.heic` | `google-heic` | 无法表达的厂商资源必须触发拒绝，不能输出 | 未测试（按设计拒绝转换） |
| A1：AVIF | 尚无固定的可转换 AVIF 样本 | `google-avif`（不支持） | CLI 拒绝目标；不可宣称提供原生写入器 | 未测试／不支持 |

可用 `cargo test --locked -p xdremux-cli --test batch_android_motion_photo` 执行自动化矩阵。H1 是测试期间由 LibLivePhoto `Input::Parts` 生成的样本，不是相机原生 HEIC。不能仅因 LibLivePhoto 提供 AVIF 配置就启用产品级 AVIF 输出。

## 准备测试文件

```bash
mkdir -p /tmp/xdremux-gallery-test/jpeg
xdremux batch --input fixtures/motion-photo/vivo/android-v1-sdr-01.jpg \
  --input fixtures/motion-photo/xiaomi/android-v1-ultrahdr-01.jpg \
  --output-dir /tmp/xdremux-gallery-test/jpeg --motion-target google-jpeg --json
sha256sum /tmp/xdremux-gallery-test/jpeg/*MP.jpg
```

为了给设备提供可导入的 HEIC 文件，可先生成 Apple 文件对，再明确传入照片和 MOV：

```bash
mkdir -p /tmp/xdremux-gallery-test/apple /tmp/xdremux-gallery-test/heic
xdremux convert --input fixtures/motion-photo/samsung/heif-ultrahdr-01.heic \
  --output /tmp/xdremux-gallery-test/apple/samsung.heic
xdremux convert --input /tmp/xdremux-gallery-test/apple/samsung.heic \
  --motion-video /tmp/xdremux-gallery-test/apple/samsung.mov \
  --motion-target google-heic \
  --output /tmp/xdremux-gallery-test/heic/samsungMP.heic --motion-report-json
```

这份导出文件经过 Apple 文件对中转，不等于 H1 单元测试生成的干净 HEIC。应分别记录输出路径与 SHA-256。若目标相册拒绝原始视频容器，可在新目录中用 `--motion-video-compat relabel-mp42` 重做 J1/J2，并单独记录结果。改写品牌并不能修复不兼容的视频编码或轨道布局。

## 实机检查项目

每个文件记录：设备型号、系统版本、屏幕 HDR 设置、相册名称及版本、输入用例、输出目标、视频兼容策略、输出 SHA-256 和传输方法。原始文件与转换后文件分开测试。在 Google Photos、OPPO 相册和已安装的三星相册中，分别记录：

1. **导入与识别：**是否识别为一张实况照片，而不是静态照片、重复照片或照片加独立视频；记录实况标记或交互入口。
2. **播放：**是否能够播放预期时长，是否卡顿、旋转错误、无声或编码不兼容；对比静态照片呈现时间附近的视频帧。
3. **HDR：**对 J2 及支持 HDR 的 HEIC 组合，在同一设备与亮度设置下比较原片和输出，记录 HDR 是否激活、亮部与 Gain Map 表现；普通截图不能独立证明 HDR 亮度。
4. **分享／导出：**从相册导出或分享一份副本，保留输出字节并使用 `xdremux inspect` 与 LibLivePhoto 严格解析，分别记录实况视频与 Gain Map 是否仍在。
5. **往返：**重新导入分享得到的副本，复查识别与播放，记录时间戳、压缩视频样本和 HDR 元数据是否保留。

每个单元格填 `PASS`、`FAIL` 或 `NOT TESTED`，附简要现象与导出文件摘要。相册出现标记不等于可以播放；可以播放也不等于支持 HDR。结论只适用于记录的版本。

## 厂商写入器启用标准

暂不启用 Samsung SEF 或其他厂商原生写入器。需要从设备原始文件证明目标相册必需的元数据布局，再用独立验证的转换文件确认相册识别、播放与导出后的资源保留。不能仅根据扩展名或相册图标推导写入规则。确定最小元数据差异并建立回归测试后，才考虑新增产品目标。

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

## 统一目标候选输出

`unified_target` CLI 测试离线覆盖下列计划。所有行的设备状态仍为未测试，包括 Android standard：历史 Google Photos 播放记录未绑定设备、应用版本与本次构建的输出。发布前，调用方必须通过 `--accept-unverified-target` 或终端交互接受预检说明。

| 目标 | 离线要求 | 设备状态 |
| --- | --- | --- |
| `android-standard`、`standard`、`google` | 别名输出完全相同的公开 Motion Photo；保留源静图容器和视频 | 未测试 |
| `samsung` | 公开 Motion Photo；保留主视频并报告未携带的 SEF/第二路视频；HEIC 的 `mpvd` 位于末尾 | 未测试 |
| `oppo` | JPEG 输入保持 JPEG。HEIF 回退为通用 HEIF，除非显式选择 JPEG | 未测试 |
| `vivo` | 保留兼容的 JPEG 单文件或写入 JPEG。HEIF 回退为通用 HEIF，除非显式选择 JPEG | 未测试 |
| `xiaomi` | JPEG 输出使用版本 1。HEIF 回退为通用 HEIF，除非显式选择 JPEG | 未测试 |
| `huawei` | 按原字节保留已有 OpenHarmony HEIC 实况容器。其他实况输入先提示，再确认是否回退到通用 Android 格式 | 未测试 |
| `honor` | 提示 XDRemux 尚无已验证的荣耀原生写入器，确认后使用通用 Android 格式 | 未测试 |
| `apple` | 现有 HEIC+MOV 文件对；Photos 识别、播放、编辑和 HDR 需要独立证据 | 未测试 |

运行 `cargo test --locked -p xdremux-cli --test unified_target` 验证这些离线路径。华为测试容器为合成文件，不能作为设备原始验收样本。

一轮本地混合批处理使用了 J1，以及用 `--target samsung` 从 H2 生成的通用 HEIC 实况。指定 `--target honor` 时，批处理在确认前停止发布。确认后分别写出 `*MP.jpg` 和 `*MP.heic`。`xdremux inspect` 读到的展示时间与输入一致，JPEG 为 1,333,944 微秒，HEIC 为 1,540,401 微秒。两组视频范围均逐字节一致，两份输入文件的摘要保持不变。

用 `--skip-existing` 重跑后，两份已发布文件的摘要保持不变。在新目录显式指定 `--motion-target google-jpeg`，得到相同的 JPEG。通用写入器报告 HEIC 项需要明确的转码路径，该项在发布前停止。

## 相册读取路径和元数据

下表记录格式及静态代码证据。可达的读取路径只适用于所列版本。生成文件的真机播放、HDR 显示和分享导出仍待验证，单独记录实测结果。

| 相册或 API | 可达的读取路径 | 拒绝条件或待确认项 | 元数据和输出处理 |
| --- | --- | --- | --- |
| 三星相册 15.1.03.4 | 先尝试 SEF `MotionPhoto_Data`，再查 XMP。XMP 路径可定位 JPEG 尾部或一直延伸到 EOF 的 32 位顶层 HEIC `mpvd`。 | JPEG XMP 路径要求视频条目含 `Item:Padding` 且 `Item:Length > 1024`。通用产物只在主图条目声明填充。HEIC 路径不处理大小为 0 或扩展大小的 box。 | 写入公开 XMP，将 HEIC `mpvd` 放在最后，并报告省略的 SEF 或次要自动播放轨。[静态追踪](https://github.com/21Z121Z1/XDRemux/pull/97) |
| 小米相册 4.3.1.15-global | JPEG 路径检查 MotionPhoto 和版本均为正数，且展示时间戳非负。版本为 1 且未进入 `MicroVideo` 分支时，从 EOF 减去 MotionPhoto 条目长度定位视频。 | 追踪到的 HEIC 标记只表示静图，HEIC 未通过 JPEG 实况偏移读取路径。 | 写入 XMP `MotionPhotoVersion=1`。已查路径未读取 `MiCamera`。保留 MP4 字节，并在 HEIC 转 JPEG 前说明损失。[静态追踪](https://github.com/21Z121Z1/XDRemux/pull/97) |
| 小米相册 4.3.1.20-global | `XmpHelper.isGoogleMotionPhoto` 要求实况标记、版本为正数，时间戳非负。版本 1 的偏移路径从 EOF 减去 MotionPhoto 条目长度。 | 共用的 XMP 解析器要求 JPEG SOI，因此这条识别及偏移路径无法读取 HEIF 主图。其他相册入口和真机播放仍待验证。 | 用户未显式选择 JPEG 时，XDRemux 将 HEIF 输入回退为通用 HEIF。[APK 来源](https://mobileosupdates.com/apps/com.miui.gallery/4030120) |
| OPPO Photos 17.10.62、25.17.35 | 追踪到的 olive photo 路径要求 JPEG MIME 和 SOI。25.17.35 检查 `MotionPhoto=1` 和可用的 XMP 容器。此前用户反馈保留视频的产物可以播放，未记录机型和相册版本。 | 追踪到的读取器没有 HEIC 路径。XMP 解析器拒绝 `MicroVideoVersion`。 | 写入公开 JPEG XMP，源文件有 `OpCamera` 才沿用。检查过的识别路径不以这些字段为必要条件。保留视频字节，并报告 HEIC 转 JPEG 的损失。[静态追踪](https://github.com/21Z121Z1/XDRemux/pull/97) [用户观察](https://github.com/21Z121Z1/XDRemux/pull/95) |
| vivo Albums 10.0.10.42、Gallery 3.2.10.4 | 10.0.10.42 是 Y04s/W20 的代理样本，含 JPEG XMP 读取路径。3.2.10.4 通过 MediaExtra `live_photo` 查询并播放独立 MP4。 | 10.0.10.42 要求 JPEG SOI，没有 `mpvd` 路径。3.2.10.4 中已追踪的相册独立 MP4 路径不会回退到单文件。其内置视频编辑器可通过 `LivePhotoMetaParser` 解析调用方提供的文件描述符、偏移及长度。相册能否从单文件定位这一范围仍待验证。S60 尚待查证。 | 两个 vivo UUID 区均存在时按原字节保留 JPEG+MP4。组合 JPEG 从源展示时间算出 `imageTime`，不生成哈希、首帧或水印元数据。[静态追踪和样本分析](https://github.com/21Z121Z1/XDRemux/pull/97) |
| 荣耀 MagicOS 10.0 | 荣耀文档说明相机保存 JPG 动态照片，图库可以播放，并可通过荣耀互联与 iPhone 双向传输。 | 通用 JPEG、HEIC 实况的识别和拒绝情况均未测试。图库文档中的同机型分享条件适用于 AI 特效。互传后的文件布局和元数据保留情况仍待验证，也仍缺可追溯的相册 APK 和相机样本。 | ZLC 记录了一份缺少版本号的 `com.hihonor.photos` 静态追踪。它从末尾为 `LIVE_N` 的三段式 60 字节尾读取长度，按 `(文件大小 - 40) - N` 定位视频。另记录了用户类型为 ` honor.org.video`（含前导空格）的 EIS UUID。两项均需用 APK 和相机原片复核。CLI 提示后使用通用 Android 格式。[荣耀拍摄和互传文档](https://www.honor.com/cn/support/content/zh-cn15889809/) [荣耀播放和编辑文档](https://www.honor.com/cn/support/content/zh-cn15897491/) [外部静态追踪](https://github.com/zsz-of/ZLC/blob/ed44963f49b1ba40d93838e90b7d18f195d09ed4/ZLivePhoto.Android/app/src/main/java/com/zsz/zlivephoto/core/formats/HonorPlugin.kt#L15-L35) |
| HarmonyOS 媒体库 | 华为公开 API 接收独立图像和视频资源，也能分别导出。固定版本的 OpenHarmony 源码另有带 `LIVE_` 尾、可选版本/帧和封面信息的组合文件导入导出路径。 | OpenHarmony 源码无法确认商用 HarmonyOS 相册的导入行为。该媒体库的 `LIVE_` 长度可包含版本/帧与 cinemagraph 数据。#97 追踪的华为 Android 40 字节尾采用另一套长度规则。 | 已识别的 Mate 70 风格 HEIC+MP4 文件按原字节保留。下游 Flutter 解析有边界限制的 MP4 `keys`/`ilst`，要求有限的大端 float32 毫秒 `covertime` 和非空 `videoId`，读取可选的 `starttime` 与 `deferredVideoEnhanceFlag`，并把封面时间转为微秒。该读取代码不代表相册可导入。[华为 API](https://developer.huawei.com/consumer/en/doc/HarmonyOS-Guides-V13/photoaccesshelper-movingphoto-V13/) [OpenHarmony 源码](https://github.com/openharmony/multimedia_media_library/blob/ad09a5f888ed78c2c46f5c218fa144ef6774e8e9/frameworks/innerkitsimpl/media_library_helper/src/moving_photo_file_utils.cpp#L237) [下游读取器](https://github.com/BeetMan/XDRemux-Flutter/blob/e66e332643d676c870ee62b2110e91869694de40/xdremux/rust/src/motion_photo.rs#L696) |
| Android Motion Photo 1.0 | 规范定义 Camera XMP、条目目录、长度、填充和展示时间。JPEG 追加媒体，HEIC 使用 `mpvd`，主图填充量等于该 box 的头长度。 | 真机接收、视频解码和 HDR 显示取决于播放端。XDRemux 无 AVIF 写入器。 | 保留主图编码、展示时间和视频字节，重新解析输出。通用写入器遇到不能映射的资源时停止，回退也遵循同一规则。[Android 格式](https://developer.android.com/media/platform/motion-photo-format) |

### HarmonyOS 封面时间与资源边界

OpenHarmony 写入组合文件时，将版本与帧记录和可选的 cinemagraph 字节计入 `LIVE_` 长度。读取时从文件总长减去声明长度和固定尾部字段定位主图边界，再拆分视频与附加数据。兼容导入会用拆分出来的视频和记录的帧索引解析封面时间。XDRemux-Flutter 的 Mate 70 路径查找 HEIF 前缀和追加的 MP4，要求存在 `ftyp`、`mdat`、`moov`、有限非负的 `covertime` 和有长度上限的 UTF-8 `videoId`，并报告解析出的元数据和视频字节范围。[OpenHarmony 读取器](https://github.com/openharmony/multimedia_media_library/blob/ad09a5f888ed78c2c46f5c218fa144ef6774e8e9/frameworks/innerkitsimpl/media_library_helper/src/moving_photo_file_utils.cpp#L659) [OpenHarmony 导入器](https://github.com/openharmony/multimedia_media_library/blob/ad09a5f888ed78c2c46f5c218fa144ef6774e8e9/services/background_task_manager/src/moving_photo_processor.cpp#L529) [下游格式识别](https://github.com/BeetMan/XDRemux-Flutter/blob/e66e332643d676c870ee62b2110e91869694de40/xdremux/rust/src/motion_photo.rs#L745)

### APK 收集记录

Android `apksig` 8.7.3 已验证 APK 签名。三星 15.1.03.4 通过 v2 验签。小米 4.3.1.20-global、OPPO 25.17.35 和 vivo 3.2.10.4 通过 v3 验签。签名证书指纹与对应镜像条目一致，小米和 vivo 则分别与表内另一版本的条目一致。

下表区分本地核验的字节与镜像条目。从 APKPure 下载的三星 15.1.03.4 和 OPPO 25.17.35 与 APKMirror 文件摘要一致。三星已复查 `com.samsung.android.motionphoto.utils.ex.d.c`、JPEG 路径 `d.d` 和 HEIF box 循环 `ex.a.d`。OPPO 的 `MediaItemExtKt.isOlivePhoto` 会同时检查 olive 标记与 JPEG MIME。其余静态结论沿用 #97 已有的追踪。

| 包名 | 版本 | APK SHA-256、已验证的签名证书 SHA-256 和来源 |
| --- | --- | --- |
| `com.sec.android.gallery3d` | 15.1.03.4 (1510300004) | APK `b99d44e31c8d198590d06090010827c38773f7a9d6560dee59ad8aaa4ceae453`，签名证书 `34df0e7a9f1cf1892e45c056b4973cd81ccf148a4050d11aea4ac5a65f900a42`。[APK 页面](https://www.apkmirror.com/apk/samsung-electronics-co-ltd/samsung-gallery/samsung-gallery-15-1-03-4-release/samsung-gallery-15-1-03-4-android-apk-download/) [APKPure](https://d.apkpure.com/b/APK/com.sec.android.gallery3d?versionCode=1510300004) |
| `com.miui.gallery` | 4.3.1.15-global (4030115) | `a726fed034edada620ff5a4e04f102080455e824c2246a5ff9368b1082d5559a` [APK 页面](https://www.apkmirror.com/apk/xiaomi-inc/miui-gallery/xiaomi-gallery-4-3-1-15-global-release/xiaomi-gallery-4-3-1-15-global-android-apk-download/) |
| `com.miui.gallery` | 4.3.1.20-global (4030120) | `0a5667969593f9df0c0faefca6cd19847074144a363d36d84f89cb0e4c7fca70`. 内嵌证书 SHA-256 为 `c9009d01ebf9f5d0302bc71b2fe9aa9a47a432bba17308a3111b75d7b2149025`。[APK 来源](https://mobileosupdates.com/apps/com.miui.gallery/4030120) |
| `com.coloros.gallery3d` | 17.10.62 (170100620) | `5118e681fa41b00e9417b3f0a9b45bac3793fa2ca8bdbd5f69e6c8cbb60a9e4c` [APK 页面](https://www.apkmirror.com/apk/coloros/oppo-photos/oppo-photos-17-10-62-release/oppo-photos-17-10-62-android-apk-download/) |
| `com.coloros.gallery3d` | 25.17.35 (25017035) | APK `e270085d137a4dccc75e2f20f4b05c4a8dabaafe9d628461d145c18d08aef2f2`，签名证书 `64aafaf1d5bc9155a9e417a849e4f8eda1d0d1341667c28ed7c443c76f820b9a`。[APK 页面](https://www.apkmirror.com/apk/coloros/oppo-photos/oppo-photos-25-17-35-release/oppo-photos-25-17-35-android-apk-download/) |
| `com.vivo.gallery` | 10.0.10.42 (100010042) | `6cb8edea5d750d0c1544d3fd7e79dd04fc388933ee14bf289b77234bc02ffe73` [APK 页面](https://www.apkmirror.com/apk/vivo-co-ltd/vivo-gallery/vivo-albums-10-0-10-42-release/vivo-albums-10-0-10-42-android-apk-download/) |
| `com.vivo.gallery` | 3.2.10.4 (30210004) | APK `94b5f58b40af41bb29eaed18a01cf03146486a6130c92b7a0877c140299241db`，签名证书 `bcc35d4d3606f154f0402ab7634e8490c0b244c2675c3c6238986987024f0c02` 与 10.0.10.42 页面一致。[APKPure](https://d.apkpure.com/b/APK/com.vivo.gallery?versionCode=30210004) |
| `com.hihonor.photos` | 150.1.1.300 候选 | 两个第三方条目的下载地址均返回 HTML。从已记录机型的设备提取主 APK 和各 split APK，记录系统版本、包名、versionCode、签名证书和文件 SHA-256。[候选条目一](https://www.962.net/azsoft/620612.html) [候选条目二](https://www.fxxz.com/azsoft/733991.html) |
| `com.hihonor.photos` | 152.0.10.794 (1520010794) | gnacho/gallery 记录了从荣耀 Magic V5 提取的 APK，报告 SHA-256 `2e8a8a89d6a9910210986935e4c01f46d3d6190936b9771520e3a13492e6ee86`。该记录未附原 APK 或签名证书，公开实现也没有复现荣耀的实况资源匹配路径。[版本报告](https://github.com/gnacho/gallery/blob/1529935f70e6d13deee98c876edf9d1a0139052e/analysis/latest/report.md#L7) [实况功能记录](https://github.com/gnacho/gallery/blob/1529935f70e6d13deee98c876edf9d1a0139052e/analysis/implementation-map.json#L658) |

小米 APKPure 最新版本下载返回 `ep-miuigallery-v15240103`（999999902），已从 4.3.1.15-global 播放证据中排除。小米 4.3.1.15-global、OPPO 17.10.62 和 vivo 10.0.10.42 的 APK 仅有镜像条目信息。

LibLivePhoto 另有带边界检查的荣耀 `LIVE_` 及反向 TLV 读取器，检查视频范围、拒绝歧义，并可读取 `HiHonor_OfflineData` 的场景模式。该读取契约与 ZLC 未固定版本的相册追踪分别记录，XDRemux 产物的相册识别仍待验证。 [LibLivePhoto 荣耀读取器](https://github.com/21Z121Z1/LibLivePhoto/blob/9328a0502bb2318b05d255ed83652c7d7c7d3b4c/crates/liblivephoto/src/vendor/honor.rs#L48)

荣耀俱乐部对帖子中标记为动态照片的一组预览（附件 `36037915`）分别提供 1024×1024 JPEG 和 MP4。JPEG 在 EOI 处结束，没有附带视频、`LIVE_` 尾部或相机型号 EXIF。独立 MP4 含 `ftyp`、`moov`、`free` 和 `mdat`。SHA-256 分别为 `61345d9378aa0b6f2ab065b66b1dde9e00221e2bddd9fdb9e463d7ca935329f7` 和 `1d0a8353998ae22ea96d5209d87429a454deb222178d5ef6c5e647e22af8ed7c`。此记录覆盖网页预览，仍需相机原片确认荣耀图库读取的布局。[荣耀俱乐部帖子](https://club.honor.com/cn/thread-30499759-1-1.html)

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

这份导出文件经过 Apple 文件对中转，不等于 H1 单元测试生成的干净 HEIC。应分别记录输出路径与 SHA-256。视频容器始终逐字节保留；若目标相册拒绝播放，应记录编码、轨道布局和实机结果，不改写容器品牌。

## 实机检查项目

为每个文件记录设备型号、系统版本、屏幕 HDR 设置、相册名称及版本、输入用例、输出目标、视频兼容策略、输出 SHA-256 和传输方法。分别在 Google Photos、三星、小米、OPPO、vivo、荣耀及 HarmonyOS 相册中测试原片、目标产物和同编码的通用回退产物，并记录以下结果。

1. **导入与识别：**是否识别为一张实况照片，而不是静态照片、重复照片或照片加独立视频；记录实况标记或交互入口。
2. **播放：**是否能够播放预期时长，是否卡顿、旋转错误、无声或编码不兼容；对比静态照片呈现时间附近的视频帧。
3. **HDR：**对 J2 及支持 HDR 的 HEIC 组合，在同一设备与亮度设置下比较原片和输出，记录 HDR 是否激活、亮部与 Gain Map 表现；普通截图不能独立证明 HDR 亮度。
4. **分享／导出：**从相册导出或分享一份副本，保留输出字节并使用 `xdremux inspect` 与 LibLivePhoto 严格解析，分别记录实况视频与 Gain Map 是否仍在。
5. **往返：**重新导入分享得到的副本，复查识别与播放，记录时间戳、压缩视频样本和 HDR 元数据是否保留。

每个单元格填 `PASS`、`FAIL` 或 `NOT TESTED`，附简要现象与导出文件摘要。相册出现标记不等于可以播放；可以播放也不等于支持 HDR。结论只适用于记录的版本。

## 厂商写入器启用标准

暂不启用 Samsung SEF 或其他厂商原生写入器。需要从设备原始文件证明目标相册必需的元数据布局，再用独立验证的转换文件确认相册识别、播放与导出后的资源保留。不能仅根据扩展名或相册图标推导写入规则。确定最小元数据差异并建立回归测试后，才考虑新增产品目标。

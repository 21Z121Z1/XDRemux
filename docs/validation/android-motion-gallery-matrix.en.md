# Android Motion Photo gallery acceptance matrix

English | [简体中文](android-motion-gallery-matrix.md)

This is a device test procedure, not a report of device acceptance. The Rust tests establish strict single-file parsing, video/timestamp preservation, checkpoint identity, and lossless resource checks. They cannot establish gallery recognition, playback, HDR rendering, or what survives a gallery export/share operation. The official [Android Motion Photo format 1.0](https://developer.android.com/media/platform/motion-photo-format) permits JPEG, HEIC, and AVIF; XDRemux's current Google product writers support only JPEG and HEIC.

## Source and output matrix

Run each supported conversion with a new output directory. Confirm the input SHA-256 against `fixtures/SHA256SUMS` before converting. Do not upload originals or converted media to CI artifacts.

| Case | Source | Command target | Portable requirement | Device status |
| --- | --- | --- | --- | --- |
| J1, SDR JPEG | `fixtures/motion-photo/vivo/android-v1-sdr-01.jpg` | `google-jpeg` | One `*MP.jpg`; strict parse, exact presentation/movie bytes | Not tested |
| J2, Ultra HDR JPEG | `fixtures/motion-photo/xiaomi/android-v1-ultrahdr-01.jpg` | `google-jpeg` | J1 plus unchanged Gain Map payload and properties | Not tested |
| H1, HEIC | Clean HEIC Motion Photo assembled by `batch_android_motion_photo.rs` from `samsung/heif-ultrahdr-01.heic` | `google-heic` | One `*MP.heic`; strict parse and unchanged movie | Not tested |
| H2, vendor-rich HEIC | `fixtures/motion-photo/samsung/heif-ultrahdr-01.heic` (unaltered) | `google-heic` | Refuse unrepresentable vendor resources; no output | Not tested (conversion intentionally blocked) |
| A1, AVIF | No pinned convertible AVIF fixture | `google-avif` (not supported) | CLI rejects target; no native writer advertised | Not tested / unsupported |

Use `cargo test --locked -p xdremux-cli --test batch_android_motion_photo` for the portable matrix. The clean H1 sample is synthesized within the test; do not mislabel its writer-produced metadata as a camera-original HEIC. A native AVIF writer must not be activated solely because the underlying library exposes an AVIF profile.

## Unified target candidates

The `unified_target` CLI tests exercise these plans offline. Every row remains untested on devices, including Android standard: historical Google Photos playback reports do not identify a device, app version, and output from this build. A caller must accept the preflight with `--accept-unverified-target` or terminal confirmation before publication.

| Target | Offline requirement | Device status |
| --- | --- | --- |
| `android-standard`, `standard`, `google` | Identical public Motion Photo output for aliases; source still container and video preserved | Not tested |
| `samsung` | Public Motion Photo; preserve the primary video and report omitted SEF/secondary streams; HEIC `mpvd` last | Not tested |
| `oppo` | JPEG for JPEG inputs. HEIF falls back to public HEIF unless JPEG is explicit | Not tested |
| `vivo` | Preserve a compatible combined JPEG or write the JPEG plan. HEIF falls back to public HEIF unless JPEG is explicit | Not tested |
| `xiaomi` | Public JPEG Motion Photo with version 1. HEIF falls back to public HEIF unless JPEG is explicit | Not tested |
| `huawei` | Preserve an existing OpenHarmony HEIC motion container byte-for-byte. Warn before falling back to Android standard for other Motion Photo inputs | Not tested |
| `honor` | Warn that XDRemux has no verified native Honor writer, then use Android standard after confirmation | Not tested |
| `apple` | Existing HEIC+MOV pair route; requires separate Photos recognition, playback, editing, and HDR evidence | Not tested |

Run `cargo test --locked -p xdremux-cli --test unified_target` for these offline paths. The Huawei test container is synthesized and is not a device-original acceptance sample.

## Gallery readers and metadata

These are format and code observations. A reachable reader is evidence for the listed build. Device playback, HDR display and share/export of the generated files remain untested unless a device observation is recorded below.

| Gallery or API | Reader path | Rejection or open question | Metadata and writer treatment |
| --- | --- | --- | --- |
| Samsung Gallery 15.1.03.4 | Tries SEF `MotionPhoto_Data` before its XMP path. The latter can locate a JPEG tail or a normal 32-bit top-level HEIC `mpvd` whose payload runs to EOF. | The traced JPEG XMP reader requires `Item:Padding` on the video item and video `Item:Length > 1024`. Standard outputs only declare padding on the primary item. Its HEIC path does not handle size 0 or extended box sizes. | The Samsung preset writes the public XMP path, places HEIC `mpvd` last, and reports omitted SEF or secondary autoplay streams. [Static trace](https://github.com/21Z121Z1/XDRemux/pull/97) |
| Xiaomi Gallery 4.3.1.15-global | The JPEG recognition path checks positive `MotionPhoto` and version plus a nonnegative presentation timestamp. With version 1 and no active `MicroVideo` branch it locates the video by subtracting the MotionPhoto item length from EOF. | The reviewed HEIC flag is for the still. The traced HEIC path cannot satisfy the JPEG motion offset reader. | Writes XMP `MotionPhotoVersion=1`. The checked path does not consume `MiCamera`. The product keeps the MP4 bytes and requires explicit JPEG selection before an HEIF-to-JPEG conversion. [Static trace](https://github.com/21Z121Z1/XDRemux/pull/97) |
| Xiaomi Gallery 4.3.1.20-global | `XmpHelper.isGoogleMotionPhoto` requires positive motion and version values plus a nonnegative timestamp. Its version-1 offset reader subtracts the MotionPhoto item length from EOF. | The shared XMP parser starts at JPEG SOI, so this recognition and offset path cannot read a HEIF primary. Other gallery entry points and device playback remain unverified. | XDRemux routes HEIF to public HEIF unless the user explicitly selects JPEG. [APK source](https://mobileosupdates.com/apps/com.miui.gallery/4030120) |
| OPPO Photos 17.10.62 and 25.17.35 | The traced olive-photo path requires JPEG MIME and SOI. The 25.17.35 reader checks `MotionPhoto=1` and a usable XMP container. A prior user report observed playback of preservation-mode output without a pinned device/app version. | The traced reader has no HEIC path. The XMP parser rejects `MicroVideoVersion`. | Public JPEG XMP with source `OpCamera` attributes when present. The attributes are optional for the checked recognition path. The product preserves video bytes and reports loss when JPEG is explicit. [Static trace](https://github.com/21Z121Z1/XDRemux/pull/97) [User observation](https://github.com/21Z121Z1/XDRemux/pull/95) |
| vivo Albums 10.0.10.42 and Gallery 3.2.10.4 | The 10.0.10.42 Y04s/W20 proxy has a JPEG XMP reader. The 3.2.10.4 trace uses a MediaExtra `live_photo` lookup and a separate MP4. | The 10.0.10.42 reader requires JPEG SOI and has no `mpvd` path. The traced 3.2.10.4 gallery sidecar path has no combined-file fallback. Its bundled video editor can parse metadata from a caller-supplied file descriptor, offset and length via `LivePhotoMetaParser`. The editor code does not establish how the gallery would discover that range in a combined file. S60 behavior remains open. | Preserve source JPEG+MP4 when both vivo UUID sections are present. The combined JPEG writer derives `imageTime` from source presentation time and does not synthesize hash, first-frame or watermark metadata. [Static trace and fixture analysis](https://github.com/21Z121Z1/XDRemux/pull/97) |
| Honor MagicOS 10.0 | Honor documents playback of local captures and same-model assets transferred with supported methods such as Honor Share on named models. | Generic JPEG/HEIC Motion Photo recognition and rejection are untested. An attributable Gallery APK and camera sample are still needed. | ZLC reports that an unversioned `com.hihonor.photos` trace finds the video at `(file size - 40) - N` from a three-part 60-byte trailer ending in `LIVE_N`. It also reports an EIS UUID with user type ` honor.org.video` (leading space). These are leads for APK and camera-file comparison. The CLI uses Android standard with a warning. [Honor playback documentation](https://www.honor.com/cn/support/content/zh-cn15897491/) [External static trace](https://github.com/zsz-of/ZLC/blob/ed44963f49b1ba40d93838e90b7d18f195d09ed4/ZLivePhoto.Android/app/src/main/java/com/zsz/zlivephoto/core/formats/HonorPlugin.kt#L15-L35) |
| HarmonyOS media library | Huawei's public API creates a moving-photo asset from separate image/video resources and can export them to separate URIs. The pinned OpenHarmony implementation also has a combined import/export path with a `LIVE_` tail, optional version/frame data and cover information. | OpenHarmony source alone cannot establish shipping HarmonyOS Gallery's import behavior. The combined media-library `LIVE_` length can include version/frame and cinemagraph data. The Android Huawei 40-byte trailer seen in PR #97 uses a distinct length contract. | Preserve recognized Mate 70-style HEIC+MP4 bytes. XDRemux-Flutter reads bounded MP4 `keys`/`ilst`, requires finite big-endian float32 `covertime` in milliseconds and a nonempty `videoId`, reads optional `starttime` and `deferredVideoEnhanceFlag`, and converts the cover time to microseconds. Its parsing does not establish what Gallery imports. [Huawei API](https://developer.huawei.com/consumer/en/doc/HarmonyOS-Guides-V13/photoaccesshelper-movingphoto-V13/) [OpenHarmony source](https://github.com/openharmony/multimedia_media_library/blob/ad09a5f888ed78c2c46f5c218fa144ef6774e8e9/frameworks/innerkitsimpl/media_library_helper/src/moving_photo_file_utils.cpp#L237) [Downstream reader](https://github.com/BeetMan/XDRemux-Flutter/blob/e66e332643d676c870ee62b2110e91869694de40/xdremux/rust/src/motion_photo.rs#L696) |
| Android Motion Photo 1.0 | The specification defines Camera XMP, an item directory, lengths, padding and presentation time. JPEG uses an appended media payload. HEIC uses the `mpvd` box and its header length for primary item padding. | Device acceptance, playback codecs and HDR rendering depend on the player. XDRemux has no AVIF writer. | Preserve source still encoding, presentation time and movie bytes. Reparse each output. Refuse unmapped resources in the standard writer, including when used as a fallback. [Android format](https://developer.android.com/media/platform/motion-photo-format) |

### APK acquisition records

Android `apksig` 8.7.3 verified the APK signatures. Samsung 15.1.03.4 verified with v2. Xiaomi 4.3.1.20-global, OPPO 25.17.35 and vivo 3.2.10.4 verified with v3. The signer fingerprints matched the corresponding mirror listings or, for the separately acquired Xiaomi and vivo versions, the listed fingerprint for the other named release.

The APK records distinguish locally verified bytes from mirror listings. Samsung 15.1.03.4 and OPPO 25.17.35 were downloaded from APKPure and matched the APKMirror file hashes. The Samsung reader was traced through `com.samsung.android.motionphoto.utils.ex.d.c`, its JPEG branch `d.d` and the HEIF box loop `ex.a.d`. OPPO’s `MediaItemExtKt.isOlivePhoto` checks the olive flag and JPEG MIME. The other code findings use the existing PR #97 traces.

| Package | Version | Listed APK SHA-256 and source |
| --- | --- | --- |
| `com.sec.android.gallery3d` | 15.1.03.4 (1510300004) | `b99d44e31c8d198590d06090010827c38773f7a9d6560dee59ad8aaa4ceae453` [APK listing](https://www.apkmirror.com/apk/samsung-electronics-co-ltd/samsung-gallery/samsung-gallery-15-1-03-4-release/samsung-gallery-15-1-03-4-android-apk-download/) [APKPure](https://d.apkpure.com/b/APK/com.sec.android.gallery3d?versionCode=1510300004) |
| `com.miui.gallery` | 4.3.1.15-global (4030115) | `a726fed034edada620ff5a4e04f102080455e824c2246a5ff9368b1082d5559a` [APK listing](https://www.apkmirror.com/apk/xiaomi-inc/miui-gallery/xiaomi-gallery-4-3-1-15-global-release/xiaomi-gallery-4-3-1-15-global-android-apk-download/) |
| `com.miui.gallery` | 4.3.1.20-global (4030120) | `0a5667969593f9df0c0faefca6cd19847074144a363d36d84f89cb0e4c7fca70`. Embedded certificate SHA-256 `c9009d01ebf9f5d0302bc71b2fe9aa9a47a432bba17308a3111b75d7b2149025`. [APK source](https://mobileosupdates.com/apps/com.miui.gallery/4030120) |
| `com.coloros.gallery3d` | 17.10.62 (170100620) | `5118e681fa41b00e9417b3f0a9b45bac3793fa2ca8bdbd5f69e6c8cbb60a9e4c` [APK listing](https://www.apkmirror.com/apk/coloros/oppo-photos/oppo-photos-17-10-62-release/oppo-photos-17-10-62-android-apk-download/) |
| `com.coloros.gallery3d` | 25.17.35 (25017035) | `e270085d137a4dccc75e2f20f4b05c4a8dabaafe9d628461d145c18d08aef2f2` [APK listing](https://www.apkmirror.com/apk/coloros/oppo-photos/oppo-photos-25-17-35-release/oppo-photos-25-17-35-android-apk-download/) |
| `com.vivo.gallery` | 10.0.10.42 (100010042) | `6cb8edea5d750d0c1544d3fd7e79dd04fc388933ee14bf289b77234bc02ffe73` [APK listing](https://www.apkmirror.com/apk/vivo-co-ltd/vivo-gallery/vivo-albums-10-0-10-42-release/vivo-albums-10-0-10-42-android-apk-download/) |
| `com.vivo.gallery` | 3.2.10.4 (30210004) | Download verified locally as `94b5f58b40af41bb29eaed18a01cf03146486a6130c92b7a0877c140299241db`. The embedded certificate SHA-256 matches the 10.0.10.42 listing. [APKPure](https://d.apkpure.com/b/APK/com.vivo.gallery?versionCode=30210004) |
| `com.hihonor.photos` | 150.1.1.300 candidate | Third-party listing only. Its endpoint did not return a usable APK. Acquire the package and any splits from a named device. Record the OS build, package ID, version code, signer SHA-256 and each file SHA-256. [Honor playback documentation](https://www.honor.com/cn/support/content/zh-cn15897491/) [APK candidate listing](https://www.962.net/azsoft/620612.html) |

The Xiaomi APKPure latest endpoint returned `ep-miuigallery-v15240103` (999999902). It was excluded from the 4.3.1.15-global playback evidence. The exact Xiaomi global, OPPO 17.10.62 and vivo 10.0.10.42 APK rows remain listing-only in this run.

LibLivePhoto also parses its own bounded Honor `LIVE_` and backward TLV variants. Its parser checks video extents and rejects ambiguous candidates, and can expose `HiHonor_OfflineData` scene mode. Those contracts are separate from the unpinned ZLC gallery trace and do not establish recognition of XDRemux outputs. [LibLivePhoto Honor reader](https://github.com/21Z121Z1/LibLivePhoto/blob/9328a0502bb2318b05d255ed83652c7d7c7d3b4c/crates/liblivephoto/src/vendor/honor.rs#L48)

## Package preparation

```bash
mkdir -p /tmp/xdremux-gallery-test/jpeg
xdremux batch --input fixtures/motion-photo/vivo/android-v1-sdr-01.jpg \
  --input fixtures/motion-photo/xiaomi/android-v1-ultrahdr-01.jpg \
  --output-dir /tmp/xdremux-gallery-test/jpeg --motion-target google-jpeg --json
sha256sum /tmp/xdremux-gallery-test/jpeg/*MP.jpg
```

For an exportable HEIC device candidate, create and inspect an Apple pair first, then explicitly provide both pair members to the Google HEIC converter:

```bash
mkdir -p /tmp/xdremux-gallery-test/apple /tmp/xdremux-gallery-test/heic
xdremux convert --input fixtures/motion-photo/samsung/heif-ultrahdr-01.heic \
  --output /tmp/xdremux-gallery-test/apple/samsung.heic
xdremux convert --input /tmp/xdremux-gallery-test/apple/samsung.heic \
  --motion-video /tmp/xdremux-gallery-test/apple/samsung.mov \
  --motion-target google-heic \
  --output /tmp/xdremux-gallery-test/heic/samsungMP.heic --motion-report-json
```

This exported sample follows an Apple-pair intermediate, unlike the synthetic H1 unit test. Record which route was used, and retain both the source and final SHA-256. The video container is always preserved byte-for-byte; if a consumer rejects it, record the codec, track layout, and device result rather than rewriting its brand.

## Device observations

For each output, record the device model, OS build, display HDR setting, gallery app/version, source case, conversion target, video policy, output SHA-256, and transfer mechanism. Test the original and converted assets separately. On Google Photos, OPPO Gallery, and Samsung Gallery where installed, record these independent results:

1. **Import and recognition.** The file appears as one Motion Photo, not a static image, duplicate, or image plus separate video. Record any Motion Photo badge or action.
2. **Playback.** Motion playback starts and covers the expected duration; note freezes, orientation, audio, or codec rejection. Compare a frame near the still's presentation timestamp.
3. **HDR.** For J2 and HEIC HDR-capable cases, compare the native original and output on the same display at the same settings. Record whether HDR is activated and whether highlight appearance or the Gain Map changes. Screenshots alone are not a reliable HDR luminance measurement.
4. **Share/export.** Export or share a copy from each gallery, preserve the exported bytes, and inspect them with `xdremux inspect` and LibLivePhoto strict parsing. Record whether the motion video and Gain Map remain; no preservation is presumed.
5. **Round trip.** Reimport the shared copy and repeat recognition/playback. Record whether timestamps, compressed video samples, and HDR metadata survived.

Mark each cell `PASS`, `FAIL`, or `NOT TESTED`, with a short observation and exported-file hash. A visual badge alone does not establish playback, and playback alone does not establish HDR support. Consumer changes are version-specific.

## Writer promotion rule

Leave Samsung SEF and other vendor-native writers disabled until a device-original target sample demonstrates a required metadata layout and an independently validated output is recognized, played, and exported with expected resources intact. Do not infer writer requirements from the source's extension or a gallery icon. Record the minimal metadata difference and create a focused regression before enabling an additional product target.

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

This exported sample follows an Apple-pair intermediate, unlike the synthetic H1 unit test. Record which route was used, and retain both the source and final SHA-256. Run J1/J2 again with `--motion-video-compat relabel-mp42` in a separate directory if a consumer rejects the unchanged container; do not imply that a brand change repairs an incompatible codec or track layout.

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

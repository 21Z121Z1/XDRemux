# Encoding quality audit summary: 2026-07-18

English | [简体中文](encoding-quality-pareto-20260718.summary.md)

The [original audit](encoding-quality-pareto-20260718.md) compared quality, encoded payload size, and encoding behavior on 2026-07-18. Its tables are historical evidence. This summary does not rerun the measurements or declare current encoder defaults.

## Measurement scope

The audit measured payloads that the implementation actively encoded. Ordinary UHDR/LHDR base HEVC payloads were copied byte-for-byte and were outside the encoding-quality comparison.

| Payload | Historical reference and metric |
| --- | --- |
| Portrait `src.image` base | Decoded source JPEG; full-resolution RGBA PSNR, MAE, and SSIMULACRA2. |
| UHDR/LHDR/Portrait Gain Map | Decoded source or reconstructed raster; code-value MAE, P99, and PSNR. |
| Styles Linear Thumbnail | Lossless generated PNG; payload size and SSIMULACRA2. |
| Neutral Style Delta Map | Constant-value reference; spatial constancy and encoded sample size. |
| Semantic masks | Raw auxiliary buffers; code error and threshold intersection-over-union. |
| Disparity | Raw auxiliary buffer; Float16 absolute error. |

Payload size means the affected HEIF item data. It does not mean an unrelated change in total file size.

## Historical selections and important limits

The audit selected the following settings for the implementation tested at that time:

| Payload | Selected setting | Recorded basis |
| --- | --- | --- |
| Portrait base | Quality `0.90` | Three real samples; quality `0.95` added 58.5% payload for 1.79 mean SSIMULACRA2 points. |
| Gain Map | Quality `0.90` | Nine real samples; quality `0.95` added 75.3% payload for a smaller absolute error reduction. |
| Gain Map tiles | 512 pixels | A 1024 tile saved 0.23% for ordinary samples but failed the Portrait graph-equivalence check. |
| Linear Thumbnail | Quality `0.85` | Three generated thumbnails; 54.3% less payload than `0.95`, with 0.294 lower mean SSIMULACRA2. |
| Neutral Delta Map | Quality `0.30` | Constant 512-pixel tile repeated 30 times; 151-byte IDR sample rather than 202 bytes at `1.0`. |
| Semantic/disparity resources | ImageIO-managed encoding | Tested primary-image quality requests did not change their payloads or decoded rasters. |

The audit also identified a padding-placement defect. Padding from height 1532 to 1536 shifted the Gain Map by four rows. On the high-detail Portrait sample, correction reduced quality-`0.90` MAE from 8.881 to 0.850 and P99 from 60 to 4.

Some requested quality values selected the same VideoToolbox tier. The tested ImageIO Gain Map path also produced identical output across requested qualities `0.80`, `0.90`, `0.95`, and `1.0`. A numeric quality argument therefore did not necessarily change a particular payload.

## Apply the findings to current work

Use the original tables for the exact historical values and sample counts. Do not extrapolate them to a different codec, operating system, image class, or resource type without new measurements.

Check current settings in the [runtime](../../crates/xdremux-runtime/src/lib.rs), [Styles runtime](../../crates/xdremux-runtime/src/apple_styles.rs), and [codec request model](../../crates/xdremux-codec/src/model.rs). The [engine policy](../../crates/xdremux-engine/src/product_policy.rs) defines current Gain Map channel and chroma selection.

The original evidence covers macOS VideoToolbox/ImageIO, parser checks, and decoded rasters. It does not establish OPPO Gallery or Apple Photos behavior on a physical device. Follow the [validation guide](README.en.md) when collecting new evidence, and keep device observations separate.

# Depth Pro 2 model adapter

English | [简体中文](README.md)

This pluggable Core ML adapter integrates Apple Depth Pro 2 (`apple/DepthPro2-coreml`) without embedding Apple model weights inside XDRemux repositories or release archives. After explicit user acceptance of the upstream [Apple Machine Learning Research License](https://huggingface.co/apple/DepthPro2-coreml/blob/4e607a682f993bbf95035054fc78ba9ffc81faf2/LICENSE), the external Core ML packages live in the user models directory (`~/Library/Application Support/XDRemux/Models`, overridable via `XDREMUX_MODEL_DIR`).

## Capabilities and boundaries

- **FOV camera estimation (`depthpro2-fov_512x896.mlpackage`)**: Samples fixed-view video frames in `xdremux-apple-adapter` and emits standard `CinematicCameraEstimate` JSON (`schema_version: 1`) for `xdremux convert --apple-cinematic` when recorded `FocalLengthIn35mmFormat` metadata is missing or unknown.
- **Still disparity inference (`depthpro2-b_w4.mlpackage` / `depthpro2-s_w4.mlpackage`)**: Supplies Core ML / Metal inverse-depth-to-disparity inference in `xdremux-apple-adapter` so `xdremux convert --apple-portrait` can convert 4:3 and 3:4 ProXDR or ISO Gain Map stills even when OPPO `rear.depth` is absent.
- **Rust owns product policy**: The Swift adapter only runs Core ML inference, orientation alignment, and ImageIO/Vision primitives. Rust owns camera admission, focal-length selection, REND generation, auxiliary packaging, validation, and atomic publication.

## Pluggable installation

Run from the repository root after reviewing the Apple AMLR license terms:

```bash
python3 scripts/distribution/install_depthpro2.py \
  --accept-apple-amlr-license \
  --models s,b,fov \
  --bin-dir /path/to/xdremux/bin
```

`install_depthpro2.py` performs three actions:

1. Downloads `depthpro2-s_w4.mlpackage`, `depthpro2-b_w4.mlpackage`, and `depthpro2-fov_512x896.mlpackage` from `apple/DepthPro2-coreml` revision `4e607a682f993bbf95035054fc78ba9ffc81faf2` into `--models-dir` and verifies every `weight.bin` SHA-256 digest.
2. Writes `.apple-amlr-license-accepted.json` in `--models-dir` so Core ML primitives can confirm explicit license consent.
3. Optionally installs the self-contained `xdremux-depthpro2` launcher into `--bin-dir` alongside `xdremux` and `xdremux-apple-adapter` without overwriting existing binaries.

## Weight SHA-256 digests (`4e607a682f993bbf95035054fc78ba9ffc81faf2`)

| Package | Role | `weight.bin` SHA-256 |
| --- | --- | --- |
| `depthpro2-s_w4.mlpackage` | Fast ANE/GPU disparity | `24b741e5608cbdd99bc2b26d4925c410b2d415637c36b1c0b51050af9c9be1db` |
| `depthpro2-b_w4.mlpackage` | Higher-detail CPU+GPU disparity | `6f47f78fcc7054239bd1f9653b82179e622b0adff0c30877b495f62bf56c37bc` |
| `depthpro2-fov_512x896.mlpackage` | Field-of-view / focal estimation | `1d95a062127b46b7d81d393aac28d58b9dec03f5535409267ee32a14ccf984f5` |

## Usage with `xdremux`

```bash
# Cinematic pending video with automatic Depth Pro 2 FOV estimation when EXIF focal length is absent
XDREMUX_DEPTHPRO2=/path/to/xdremux/bin/xdremux-depthpro2 \
  xdremux convert --input fixed-view.mov --output pending.mov --apple-cinematic

# Or run the estimator directly and pass the JSON estimate
python3 model_adapters/depthpro2/estimate.py --input fixed-view.mov --output camera.json
xdremux convert --input fixed-view.mov --output pending.mov \
  --apple-cinematic --cinematic-camera-estimate camera.json

# 4:3 / 3:4 still photo to Apple Portrait using Depth Pro 2 + Vision/ImageIO
xdremux convert --input photo_4x3.heic --output portrait.heic --apple-portrait
```

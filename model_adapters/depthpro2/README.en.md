# Depth Pro 2 model adapter

English | [简体中文](README.md)

The Depth Pro 2 adapter uses locally installed Apple model weights for Core ML and PyTorch. On macOS the default model directory is `~/Library/Application Support/XDRemux/Models`. The standalone PyTorch adapter uses `~/.local/share/XDRemux/Models` on Linux. Set `XDREMUX_MODEL_DIR` to choose a different directory. Review the license before installing either backend. [Apple Machine Learning Research Model License](https://huggingface.co/apple/DepthPro2-S/blob/903a1c1ade918831d138adcc4aedbcc8f4da0078/LICENSE)

## Generate half-resolution depth with PyTorch

The S and B models accept still images at any aspect ratio. The command writes a Float32 NumPy array (`.npy`) with depth in meters along the optical axis. Width and height are each halved and rounded up after applying EXIF orientation. A `1001×751` image yields a `501×376` map. Pixels at infinite depth carry `+inf`. JPEG, PNG, HEIC and the other still formats supported by Pillow can be used.

Use a Python 3.11+ virtual environment. Install a PyTorch wheel compatible with your hardware before installing the adapter requirements. [PyTorch installation](https://pytorch.org/get-started/locally/)

```bash
python3 -m venv ~/.venvs/xdremux-depthpro2
source ~/.venvs/xdremux-depthpro2/bin/activate
python -m pip install -r model_adapters/depthpro2/requirements-pytorch.txt

python scripts/distribution/install_depthpro2.py --backend pytorch \
  --accept-apple-amlr-license --models s,b,fov
python model_adapters/depthpro2/pytorch_depth.py --model s \
  --input photo.heic --output depth.npy
```

The installer puts the weights and license receipt in the model directory's `pytorch` subdirectory. It checks the official S and B SHA-256 digests and the size of Apple's shared FOV checkpoint. Supply `--focal-length-px` for a calibrated focal length in original oriented pixels. Otherwise the helper uses the image's 35mm-equivalent EXIF focal length when available and loads the FOV checkpoint on demand.

By default the adapter runs inference on a half-resolution copy and caps its longer edge at 1024 pixels. It preserves framing, resamples the inverse depth to the requested half resolution, and performs any output upscaling on the CPU. A lower cap reduces memory and can soften depth edges. Use `--native --max-side 0` to infer at the original size before producing the same half-resolution map. Native inference uses more memory. For inputs that need 32-pixel alignment, the model temporarily repeats the last RGB row and column and trims the prediction back to the working image size.

Device selection prefers CUDA, then MPS, then CPU. `--device` and `--precision` override the defaults. CUDA uses BF16 where supported and FP16 otherwise. MPS uses BF16 and CPU uses FP32. Repeat `--input` and `--output` in matching order to process independent images with one model load.

### Use PyTorch for Apple Portrait

Install the optional launcher into an existing bin directory by adding `--bin-dir` to the installation command. On macOS, use the launcher for a supported 4:3 or 3:4 HDR still that needs synthesized depth.

```bash
XDREMUX_DEPTHPRO2_PYTORCH=/path/to/xdremux-depthpro2-torch \
XDREMUX_DEPTHPRO2_PYTORCH_MODEL=b \
  xdremux convert --input photo_4x3.heic --output portrait.heic --apple-portrait
```

The launcher prefers S when both models are installed. Set `XDREMUX_DEPTHPRO2_PYTORCH_MODEL` to `s` or `b` to select one. XDRemux uses source OPPO depth when present. The Portrait conversion still needs the macOS Vision and ImageIO adapter for its semantic mattes and auxiliary resources.

## Capabilities and boundaries

- **FOV camera estimation (`depthpro2-fov_512x896.mlpackage`)**: Samples fixed-view video frames in `xdremux-apple-adapter` and emits standard `CinematicCameraEstimate` JSON (`schema_version: 1`) for `xdremux convert --apple-cinematic` when recorded `FocalLengthIn35mmFormat` metadata is missing or unknown.
- **Still disparity inference (`depthpro2-b_w4.mlpackage` / `depthpro2-s_w4.mlpackage`)**: Supplies Core ML / Metal inverse-depth-to-disparity inference in `xdremux-apple-adapter` so `xdremux convert --apple-portrait` can convert 4:3 and 3:4 ProXDR or ISO Gain Map stills even when OPPO `rear.depth` is absent.
- **Rust owns product policy**: The Swift adapter only runs Core ML inference, orientation alignment, and ImageIO/Vision primitives. Rust owns camera admission, focal-length selection, REND generation, auxiliary packaging, validation, and atomic publication.

## Install Core ML models

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

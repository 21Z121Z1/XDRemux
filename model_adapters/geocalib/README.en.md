# GeoCalib model adapter

English | [简体中文](README.md)

This local model primitive runs GeoCalib on sampled frames of a fixed-view SDR video. The Rust runtime automatically invokes it when a Cinematic input has no fixed recorded 35mm-equivalent focal length. It estimates each frame independently, fits one shared camera, and returns source-bound pixel intrinsics and uncertainty. The Rust engine admits the observations and selects one constant effective equivalent. The Apple adapter uses that value for transient disparity inference and every timed rendering sample. The model helper does not select conversion policy or generate MOV files.

The model does not recover physical lens calibration, capture aperture, focus distance, Apple focus disparity or stabilization crop. It does not provide depth maps to Photos. Single-image camera estimation can be wrong even when uncertainty is low. Sample checks do not prove that unsampled frames share the same geometry. Split edited or zooming video into fixed-view clips before using this initial path.

## Setup and use

Install ExifTool for automatic recorded-metadata probing, plus FFmpeg and FFprobe for model frame decoding, on `PATH`. Use a separate Python environment. The tested inference path uses CPU on Apple silicon with Python 3.13, PyTorch 2.14.1 and torchvision 0.29.1. From the repository root:

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

Replace the example paths with persistent locations. `--bin-dir` must exist and normally contains `xdremux` and `xdremux-apple-adapter`. The installer checks the pinned weights hash and publishes a self-contained `xdremux-geocalib` launcher without overwriting an existing helper. The launcher embeds the model primitive and refers to the selected Python environment and weights by absolute path; it does not depend on the source checkout after installation. Keep that environment and weights in place. Product archives do not bundle these dependencies. Conversion never installs packages or downloads weights automatically.

Rust looks for ExifTool and GeoCalib beside the Apple adapter, then on `PATH`. `XDREMUX_EXIFTOOL` and `XDREMUX_GEOCALIB` override the executable paths. Explicit `--focal-length-35mm` bypasses both tools. Automatic metadata admission uses only main-asset EXIF/XMP `FocalLengthIn35mmFormat` or QuickTime `camera.focal_length.35mm_equivalent`, excluding physical focal length, composites and thumbnails. Zero is unknown. Conflicting or malformed known values fail; missing or unknown values invoke GeoCalib. Neither path silently substitutes 24 mm.

The model runs locally and uploads no media. The default is five uniformly spaced sampling targets. It decodes the first video stream with its display orientation, scales the complete frame to at most 1280 pixels per edge without cropping, and records intrinsics in those coordinates. It rejects HDR transfer functions, non-square source pixels and attached-picture streams. Rust validates the generated JSON before invoking the native adapter and removes the private per-conversion JSON when the operation ends. Model failure, missing dependencies or rejected observations fail without publishing a MOV.

The helper requires GeoCalib revision `1dedb29f94fc11b896c5f572005978aeebccd67f` and checks installed package provenance. Release weights must have SHA-256 `86d6aeacd8bbd974c59ce39f61854e00d36911c732ad89be471476fd708722ac`. Both input and weights are hashed before and after inference. Rust compares the estimate source hash to the same source hash it rechecks before MOV publication.

For reproducible diagnostic observations, the primitive can still run directly:

```bash
/path/to/camera-env/bin/python model_adapters/geocalib/estimate.py \
  --input fixed-view.mov --weights /path/to/geocalib-pinhole.tar --output camera.json
xdremux convert --input fixed-view.mov --output pending.mov \
  --apple-cinematic --cinematic-camera-estimate camera.json
```

`--samples` accepts 3 through 128 for direct diagnostic use. Existing JSON files are never overwritten. `--cinematic-camera-estimate` is available only on `convert` and requires `--apple-cinematic`; it overrides automatic metadata/model selection. An explicit `--focal-length-35mm` takes priority without reading the JSON. Aperture retains its separate `--aperture` option and f/2.8 default.

Prefer known source-camera geometry or an explicit capture prior over image-only estimates. For a uniform stabilization crop factor c, the effective view-angle equivalent scales approximately as nominal_equivalent_mm * c. A crop narrows the view and increases this equivalent; it does not reduce a known 70 mm view to 40 mm. An unknown EIS crop cannot be recovered from the nominal focal length alone.

## Observation contract

Schema version 1 contains `source_sha256`, an `estimator` object with `name`, `revision` and `weights_sha256`, and `samples`. Each sample contains a sampling target `time_seconds` and an `intrinsics` object. The optional `shared_intrinsics` object carries a joint fixed-view fit. Each intrinsics object contains `image_size` as `[width, height]`, `focal_length_pixels` as `[fx, fy]`, `principal_point_pixels` as `[cx, cy]`, and `focal_uncertainty_pixels`. The principal point is in pixels. GeoCalib assumes it is centered and does not independently recover it. The uncertainty is GeoCalib's focal uncertainty in the sampled image's pixel units. Other estimators must supply the same units and semantics rather than inserting a confidence score.

Rust rejects JSON larger than 1 MiB, unsupported schemas, missing provenance, a source-hash mismatch, or fewer than 3 or more than 128 samples. Sampling targets must be finite, nonnegative and strictly increasing. Dimensions must be 1 through 16384. Focal values must be positive and finite, with `fx/fy` within 1 percent of 1. The principal point must be within one pixel of image center. All uncertainties must be finite and nonnegative. The selected shared fit must have uncertainty at most 20 percent of `fy`. Without a shared fit, every independent sample must meet that bound. These are conservative admission checks, not calibrated accuracy guarantees.

For each accepted sample, Rust computes:

```text
equivalent_mm = hypot(36, 24) / hypot(width / fx, height / fy)
```

This matches the diagonal field of view to a virtual 36 by 24 mm frame. It is invariant to uniform resize and 90-degree rotation. It is an effective view-angle estimate, not the physical focal length of the source camera. Each result must be within the native 1 through 300 mm range. Rust selects the shared fit when present, or the median of independent samples otherwise. Every independently estimated focal length must be within 20 percent of the selected value. A shared fit cannot hide conflicting frame estimates. A failed shared fit does not fall back to independent estimates. It rejects uncertain or inconsistent observations before invoking the Apple adapter and does not silently use 24 mm after an estimate fails.

The pinned GeoCalib revision's shared-mode uncertainty accessor uses covariance index 2, which belongs to the second frame's gravity in a multi-frame fit. The joint pinhole system orders the `2 * sample_count` gravity parameters before its single focal parameter. This tool reads that last focal variance and applies GeoCalib's square-root/2 convention, then undoes the vertical preprocessing scale. It verifies the covariance dimensions. It does not treat the upstream gravity-derived number as focal certainty. PyTorch's random seed is fixed at zero for the model's randomized basis generation.

The source hash used for admission is the same hash the runtime rechecks before publication. The runtime also requires the adapter to report the requested focal length and aperture. Existing output protection and pending-state native validation remain part of conversion. Model execution, Rust admission, native preprocessing and Photos editing are separate evidence layers. A new estimate or successful pending MOV does not establish improved blur quality or physical camera accuracy.

GeoCalib is maintained by its authors at [cvg/GeoCalib](https://github.com/cvg/GeoCalib). See its [code license](https://github.com/cvg/GeoCalib/blob/1dedb29f94fc11b896c5f572005978aeebccd67f/LICENSE) and [weights license](https://github.com/cvg/GeoCalib/blob/1dedb29f94fc11b896c5f572005978aeebccd67f/LICENSE-WEIGHTS). XDRemux does not redistribute the model in its product package.

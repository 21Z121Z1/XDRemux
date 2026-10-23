# Cinematic camera observations

English | [简体中文](README.md)

This optional research tool runs GeoCalib on sampled frames of a fixed-view SDR video. It estimates each frame independently and then fits one shared camera to the same frames. It writes camera observations for `xdremux convert --apple-cinematic --cinematic-camera-estimate`. The Rust engine validates the observations and selects a constant effective 35mm-equivalent focal length. The existing Apple adapter uses that value for transient disparity inference and every timed rendering sample.

The tool does not recover physical lens calibration, capture aperture, focus distance, Apple focus disparity, or stabilization crop. It does not provide depth maps to Photos. Single-image camera estimation can be wrong even when its uncertainty is low. The sample checks do not prove that unsampled frames have the same camera geometry. Split edited or zooming video into fixed-view clips before using this initial path.

## Setup and use

Use a separate Python environment. The tested inference path uses CPU on Apple silicon with Python 3.13, PyTorch 2.14.1 and torchvision 0.29.1. Install the pinned source and direct dependencies with:

```bash
python3 -m venv camera-env
camera-env/bin/python -m pip install -r research/cinematic_camera/requirements.txt
curl --fail --location https://github.com/cvg/GeoCalib/releases/download/v1.0/geocalib-pinhole.tar --output geocalib-pinhole.tar
camera-env/bin/python research/cinematic_camera/estimate.py \
  --input fixed-view.mov --weights geocalib-pinhole.tar --output camera.json
xdremux convert --input fixed-view.mov --output pending.mov \
  --apple-cinematic --cinematic-camera-estimate camera.json
```

The model and weights run locally. No media is uploaded. The estimator requires FFmpeg and FFprobe. Its default is five uniformly spaced sampling targets. `--samples` accepts 3 through 128. It decodes the first video stream with its display orientation, scales the complete frame to at most 1280 pixels per edge without cropping, and records intrinsics in those image coordinates. It currently rejects HDR transfer functions, non-square source pixels and attached-picture streams. It does not overwrite an existing JSON file. The CLI does not install or launch Python.

The tool requires GeoCalib source revision `1dedb29f94fc11b896c5f572005978aeebccd67f` and checks the installed package's source provenance. It requires release weights with SHA-256 `86d6aeacd8bbd974c59ce39f61854e00d36911c732ad89be471476fd708722ac`. The profile records the source-video hash, model revision and weights hash. Both input and weights are hashed again after inference. Weights are downloaded separately and are not included in XDRemux.

`--cinematic-camera-estimate` is available only on `convert` and requires `--apple-cinematic`. An explicit `--focal-length-35mm` takes priority and the estimate file is not read. Without either option, the existing 24 mm assumption remains. Aperture retains the separate `--aperture` option and its f/2.8 default.

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

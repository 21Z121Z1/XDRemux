# Pending Cinematic MOV

English | [简体中文](cinematic.md)

The `xdremux cinematic` command creates a new pending Cinematic MOV from ordinary video. The acceptance target is the public Apple `needsPreprocessing` state. The command does not claim that inferred effects match the original camera's effects.

## Command and requirements

```bash
xdremux cinematic --input ordinary.mp4 --output pending.mov
```

The output must not exist. The input remains unchanged. The Rust runtime hashes the input before and after generation. It publishes the candidate with a hard link that cannot replace an existing output.

The current implementation requires macOS 27, available Apple inference resources, and the private Portrait/Cinematic framework methods used by the adapter. The adapter checks each required Objective-C method encoding before use. An unsupported method or resource causes failure.

`--focal-length-35mm MM` sets the initial focal-length assumption. It defaults to 24. Its permitted range is 1 to 300. `--aperture F` defaults to 2.8. Its permitted range is 1 to 64. Both values must be finite. They are capture assumptions when the source lacks camera geometry, not measured optical properties.

## Native generation path

1. Decode the selected color-video track with AVFoundation.
2. Estimate relative image magnification with Vision homographic registration. Apply accepted scale changes to the initial focal-length assumption. Use Apple's `PTMonocularDisparityProvider` to infer transient disparity from each decoded frame with that frame's assumed focal length. Detect faces with Vision. When no face is present, detect upper bodies. Give Vision the video display orientation and map its rectangles back to encoded coordinates. Prefer a large subject and retain a nearby prior anchor when possible. When both detectors find no subject, select a confident objectness-saliency region. Use a center region if saliency supplies no suitable region. Object and center regions use a synthetic script anchor; they do not assert that a person was detected.
3. Use Apple's track/focus allocators and `PTCinematographyPostcaptureRefinement` to create and serialize a coherent frame script.
4. Write new global Cinematic metadata and one timed Cinematic metadata group for each decoded frame. Set unknown capture focus/disparity to the native pending sentinel `-1`. Set visible crop to `(1,1)` so Apple's parser retains the per-frame assumed focal length. Set the virtual sensor region to the encoded image size. Write aperture to both the script and timed rendering data.
5. Copy compressed video and all supported audio tracks into the MOV. Preserve the video transform and track timescale. Discard transient inference buffers after metadata generation. Do not write a raster disparity track.
6. Load the new MOV with public `CNAssetInfo`. Require `needsPreprocessing`, resource status `ready`, and `isPreprocessed == false` before the runtime publishes the file.

The global rendering block uses a compatible virtual capture profile and rendering version 5. Hardware/sensor fields, the virtual sensor region, and the luma-derived scene-illuminance proxy are synthetic. They do not recover the source camera calibration, stabilization crop, or original focus choices. No native sample's Cinematic payload or disparity track is copied into the output.

Relative magnification comes only from ordinary decoded video. Registration uses reduced luma frames at approximately 5 Hz. The generator accepts substantial, spatially consistent scale changes and rejects small drift. It limits each frame's assumed focal length to 1 through 300. This estimate cannot distinguish all camera motion from lens zoom. It cannot determine absolute field of view or recover the original camera's optical calibration. A failed registration keeps the prior estimate. A new shot does not reset the estimate. Original Cinematic geometry and focus decisions are diagnostic references only.

The Rust CLI remains the product entry point. The Swift/Objective-C adapter performs Apple framework operations. The output is self-contained.

## Verification

Private samples can remain outside Git. Run the following command from this checkout. Use new result and build directories, or reuse existing build directories. The result directory must not exist.

```bash
python3 scripts/verify_cinematic_envelope.py \
  --input /absolute/path/native.MOV \
  --run-dir /absolute/path/new-results \
  --swift-build-dir /absolute/path/swift-build \
  --rust-build-dir /absolute/path/rust-build \
  --synthetic --compare-render
```

Repeat `--input` for more native samples. The runner builds the current CLI and adapter. It removes all non-video/non-audio tracks and global metadata from each native sample. Public `CNAssetInfo` must report `none` for this ordinary derivative. The runner then invokes the CLI to generate an independent pending envelope.

The public consumer check validates the script, selected detection IDs, frame count, timestamps, aperture, timed keys, rendering frame attributes, virtual sensor geometry, per-frame focal-length range, and pending resource state. It records the focal-length timeline. FFprobe compares individual encoded video/audio packets, hashes, timestamps, and audio trim data. AVAssetReader audio buffer grouping is not a packet-identity test. The runner checks source hashes again after each operation. `--synthetic` adds an H.264 input with no original Cinematic data or detectable faces.

Test relative magnification with generated RGB videos that have known 2× zoom and pure translation:

```bash
python3 scripts/verify_cinematic_inference.py \
  --cli /absolute/path/rust-build/debug/xdremux \
  --adapter /absolute/path/swift-build/out/Products/Debug/xdremux-apple-adapter \
  --probe /absolute/path/new-results/cinematic-consumer-probe \
  --run-dir /absolute/path/new-inference-results
```

This check invokes the real CLI and public consumer. The zoom result must approach twice the initial focal-length assumption. Translation must not change that assumption by more than 2%. Both outputs must retain the encoded media unchanged and enter `needsPreprocessing`.

## Headless effect comparison

`--compare-render` invokes public `CNAssetInfo` preprocessing separately for the native and regenerated asset. Both must become `renderable`. It then uses public `CNRenderingSession` at export quality to render frames at 25%, 50%, and 75% of the common duration, at f/2 and f/16. Both assets use the same decoded color, timestamps, 10-bit input format, output geometry, and identity display transform.

Measurements use full-resolution `RGBA16Float` texture RGB values. The native RGBA destination retains the source transfer function. For HLG input, these values are HLG-encoded BT.2020 RGB. They are not linear sRGB. The tool also converts the rendered image to extended linear BT.2020 and records separate linear render and effect differences. Both measurements exclude alpha. The tool hashes decoded color planes and requires identical inputs. The report records exact pixel identity, mean absolute error, root mean square error, peak signal-to-noise ratio, and the fraction of components with an absolute difference above 0.001. It also records each focus rectangle. It compares inferred disparity and the f/2 versus f/16 effect across the full frame and within the native focus rectangle. Effect cosine similarity measures whether the blur changes occur in similar areas. A repeated native render measures determinism.

The generated render with a native focus value is a diagnostic control. Disparity scale and offset are undefined. A shared number does not prove that two different maps use the same physical focus plane.

To compare a new verification run with an earlier run, use:

```bash
python3 scripts/compare_cinematic_alignment.py \
  --baseline /absolute/path/baseline/report.json \
  --current /absolute/path/current/report.json \
  --output /absolute/path/current/alignment.json \
  --require-improvement SAMPLE_NAME --minimum-reduction 0.25
```

The tool requires the current report to match `HEAD`. It checks source hashes, ordinary-input integrity, media packet identity, decoded color identity, sample times, render settings, native repeatability, and unchanged reference effect energy. The required sample must reduce both mean f/2 render error and mean relative effect error. The report also shows every other sample, including regressions. This check does not turn a measured improvement into an equivalence guarantee.

Earlier reports incorrectly labeled the unconverted texture values as `raw linear RGB`. Their encoded RGB measurements can be compared with current encoded RGB measurements only when decoded color hashes match and native reference effect energy is unchanged. The comparison tool records this label correction. Earlier PNG previews that interpreted HLG textures as linear sRGB do not provide a valid visual reference.

PNG files are SDR previews. Core Image uses the decoded buffer's color space, content headroom, and average light level. `CISystemToneMap` maps the image to standard dynamic range with display headroom 1. The tool applies the source display transform and writes 8-bit PNG with an sRGB profile. These previews are not floating point measurement data. They do not establish equality with Dolby Vision dynamic-trim playback in Photos. Test preprocessing derivatives reference their unchanged input color tracks to avoid duplicate data. These test derivatives are not portable; the product `pending.mov` remains self-contained.

Run the HDR color regression with the renderer built by the verification runner:

```bash
/absolute/path/new-results/cinematic-render-comparison \
  --verify-color /absolute/path/new-color-results
```

Known HLG patches must match a color-managed reference. The former linear-sRGB interpretation must fail the negative control. The SDR PNG must have an sRGB profile and distinct highlight levels after system tone mapping.

A completed comparison is not an equivalence result. Use `all_compared_frames_identical` and the numerical differences to assess equivalence. Missing camera calibration, optical geometry, or original focus decisions can produce differences even when both assets render successfully. This test does not prove Apple Photos save/reopen behavior, device Gallery behavior, or equality at unsampled frames.

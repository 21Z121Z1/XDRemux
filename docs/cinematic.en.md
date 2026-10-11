# Pending Cinematic MOV

English | [简体中文](cinematic.md)

The `xdremux cinematic` command creates a new pending Cinematic MOV from ordinary video. The acceptance target is the public Apple `needsPreprocessing` state. The command does not claim that inferred effects match the original camera's effects.

## Command and requirements

```bash
xdremux cinematic --input ordinary.mp4 --output pending.mov
```

The output must not exist. The input remains unchanged. The Rust runtime hashes the input before and after generation. It publishes the candidate with a hard link that cannot replace an existing output.

The current implementation requires macOS 27, available Apple inference resources, and the private Portrait/Cinematic framework methods used by the adapter. The adapter checks each required Objective-C method encoding before use. An unsupported method or resource causes failure.

`--focal-length-35mm MM` defaults to 24. Its permitted range is 1 to 300. `--aperture F` defaults to 2.8. Its permitted range is 1 to 64. Both values must be finite. They are capture assumptions when the source lacks camera geometry, not measured optical properties.

## Native generation path

1. Decode the selected color-video track with AVFoundation.
2. Use Apple's `PTMonocularDisparityProvider` to infer transient disparity from each decoded frame. Use Vision detections or a center fallback to establish focus anchors.
3. Use Apple's track/focus allocators and `PTCinematographyPostcaptureRefinement` to create and serialize a coherent frame script.
4. Write new global Cinematic metadata and one timed Cinematic metadata group for each decoded frame. Set unknown capture focus/disparity to the native pending sentinel `-1`. Set visible crop to `(1,1)` so Apple's parser retains the assumed focal length. Write aperture to both the script and timed rendering data.
5. Copy compressed video and all supported audio tracks into the MOV. Preserve the video transform and track timescale. Discard transient inference buffers after metadata generation. Do not write a raster disparity track.
6. Load the new MOV with public `CNAssetInfo`. Require `needsPreprocessing`, resource status `ready`, and `isPreprocessed == false` before the runtime publishes the file.

The global rendering block uses a compatible virtual capture profile and rendering version 5. Hardware/sensor fields and the luma-derived scene-illuminance proxy are synthetic. They do not recover the source camera calibration. No native sample's Cinematic payload or disparity track is copied into the output.

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

The public consumer check validates the script, selected detection IDs, frame count, timestamps, aperture, timed keys, rendering frame attributes, and pending resource state. FFprobe compares individual encoded video/audio packets, hashes, timestamps, and audio trim data. AVAssetReader audio buffer grouping is not a packet-identity test. The runner checks source hashes again after each operation. `--synthetic` adds an H.264 input with no original Cinematic data or detectable faces.

## Headless effect comparison

`--compare-render` invokes public `CNAssetInfo` preprocessing separately for the native and regenerated asset. Both must become `renderable`. It then uses public `CNRenderingSession` at export quality to render frames at 25%, 50%, and 75% of the common duration, at f/2 and f/16. Both assets use the same decoded color, timestamps, 10-bit input format, output geometry, and identity display transform.

Measurements use full-resolution `RGBA16Float` texture RGB values. They exclude alpha. The report records exact pixel identity, mean absolute error, root mean square error, peak signal-to-noise ratio, and the fraction of components with an absolute difference above 0.001. It also compares inferred disparity and the f/2 versus f/16 effect. A repeated native render measures determinism. A generated render with the native focus value isolates part of the focus difference.

PNG files are 8-bit sRGB previews. They are not the floating point measurement data. Test preprocessing derivatives reference their unchanged input color tracks to avoid duplicate data. These test derivatives are not portable; the product `pending.mov` remains self-contained.

A completed comparison is not an equivalence result. Use `all_compared_frames_identical` and the numerical differences to assess equivalence. Missing camera calibration, optical geometry, or original focus decisions can produce differences even when both assets render successfully. This test does not prove Apple Photos save/reopen behavior, device Gallery behavior, or equality at unsampled frames.

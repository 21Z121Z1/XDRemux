# Pending Cinematic MOV

English | [简体中文](cinematic.md)

The `xdremux convert --apple-cinematic` path creates a new pending Cinematic MOV from ordinary video. The acceptance target is the public Apple `needsPreprocessing` state. The command does not claim that inferred effects match the original camera's effects.

## Command and requirements

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
```

The flag is available on `convert` and `batch`, like the Portrait and Photographic Styles flags. It is exclusive with image-feature and OPPO compatibility flags. The output must use `.mov` and must not exist. Without an explicit single-file output, `convert` creates `SOURCE.xdremux.mov` beside the source. The input remains unchanged. The Rust runtime hashes the input before and after generation. It publishes the candidate with a hard link that cannot replace an existing output.

The current implementation requires macOS 27, a macOS 27 SDK build, available Apple inference resources, and the private Portrait/Cinematic framework methods used by the adapter. The adapter checks each required private Objective-C method encoding before use. An unsupported method, SDK, OS, or resource causes failure. Older SDK builds retain the other Apple adapter operations and reject this command before inference.

This is an experimental pending-metadata command. `needsPreprocessing` and `ready` confirm that Apple recognizes the metadata; they do not guarantee that Apple's later preprocessing supports every input. On macOS 27.0.1, stripped 1080p/4K HEVC camera inputs and independently encoded 1080p H.264 inputs completed public preprocessing, rendering, and edit reload. A matched 1920×1080, 30 fps short-video test also passed with H.264, HEVC, and ProRes Standard. HEVC is not a required input codec for the public processing path. An independent 640×360 H.264 input reached the pending state but preprocessing returned `CNCinematicErrorDomain` code 5 (`unsupported`). Changing its color tags or virtual sensor metadata did not resolve the native size rejection. The command preserves encoded media and does not resize or transcode to work around that restriction. Other dimensions, codecs, systems, and iPhone Photos processing require their own consumer verification.

`--focal-length-35mm MM` sets the focal-length assumption for every frame. It defaults to 24. Its permitted range is 1 to 300. `--aperture F` defaults to 2.8. Its permitted range is 1 to 64. Both values must be finite. Supplying either option requires `--apple-cinematic`. They are capture assumptions when the source lacks camera geometry, not measured optical properties.

`convert --cinematic-camera-estimate camera.json` supplies optional source-bound pinhole observations. Rust checks their provenance, uncertainty and consistency, then selects a constant effective view-angle equivalent. An explicit `--focal-length-35mm` overrides the estimate without reading it. The default remains 24 mm when neither is supplied. This initial option supports fixed-view clips on `convert`; it does not estimate a zoom timeline. See the [camera observation contract and GeoCalib tool](../research/cinematic_camera/README.en.md).

## Minimum processing information

The codec is decoded before inference. An ordinary video still needs a valid Cinematic envelope, even if its codec is supported. For the tested public path, the envelope contains a global `com.apple.quicktime.cinematic-video` block, per-frame cinematography with valid detection IDs and focus decisions, and per-frame rendering with camera-geometry assumptions. Video and metadata must describe the same visible timeline. A raster disparity track is not required for `needsPreprocessing`.

A diagnostic key-ablation test renamed metadata keys without changing media bytes or container offsets. Disabling the cinematography key changed capability to `none`. Disabling the global block left a pending asset with resources not ready. Disabling timed rendering left a ready pending asset whose processing failed. This confirms that pending classification alone does not validate all processing inputs.

On the tested 1080p H.264 short clip, jointly disabling the recognized intent, `metadata.1`, aperture/disparity scalar, and scene-illuminance keys still passed public preprocessing, rendering, and edit reload. The retained rendering/script blocks still carried their own values. An autofocus-only script with one initial decision also passed. These tests do not remove those fields from production output or establish the minimum schema for Photos. They do not recover physical camera autofocus, original capture calibration, or missing depth from a codec.

## Native generation path

1. Decode the selected color-video track with AVFoundation.
2. Use Apple's `PTMonocularDisparityProvider` to infer transient disparity from each decoded frame with the resolved focal length. Detect faces with Vision. When no face is present, detect upper bodies and body-pose landmarks. If both ear landmarks have confidence at least 0.3, derive a head seed from their midpoint and separation. Use 1.4 times their horizontal separation for the width, with a permitted normalized width of 0.01 through 0.5. Convert a square display-pixel region to normalized coordinates with the oriented image aspect ratio. Prefer up to two such head seeds. Otherwise use the upper-body seeds. Give Vision the video display orientation and map its rectangles back to encoded coordinates. When no human seed remains, use Vision animal recognition with label confidence at least 0.5. Match dog/cat head landmarks to their recognized body. A head needs a nose and at least one eye with confidence 0.3, at least three confident head points, and at least 60% overlap with its body. Otherwise seed the body. Only then fall back to the objectness-saliency region with the highest confidence. Its confidence must be at least 0.3. Its area must be 0.5% through 60% of the image. Mark object seeds as `unknown`; they do not assert that a person was detected.
3. Use public `CNObjectTracker` to continue up to two subjects through detector gaps while its bounds and confidence remain valid. Match detections to these subjects. Allocate a new track/group for a new subject. Keep each track's detection type stable. Animal matching uses both its head region and recognized body region, and keeps dog/cat identities separate from generic saliency. Retain a tracked animal head through pose misses while the tracker remains valid. Reuse each animal part's track ID within the actor group when it returns; human type reseeds start a new track as before. Use a matched human or animal head detector region to correct tracker drift. Retain a valid selected subject unless a detected head has more than 1/0.6 times its rectangle area. Prefer a human over a non-human seed. Select a center autofocus region when no subject remains. Keep the autofocus track separate from subject tracks. Use Apple's recorded track/focus allocator namespaces, leaving public editor namespaces free. Generate a base decision at each subject change. Use `PTCinematographyPostcaptureRefinement` to create and serialize the script. During a transition, keep the refined focus detection and frame header consistent. Store the base selection separately.
4. Write new global Cinematic metadata and one timed Cinematic metadata group for each decoded frame. Set unknown capture focus/disparity to the native pending sentinel `-1`. Set visible crop to `(1,1)` so Apple's parser retains the per-frame assumed focal length. Set the virtual sensor region to the encoded image size. Write aperture to both the script and timed rendering data.
5. Copy compressed video and all supported audio tracks into the MOV. Preserve the video transform and track timescale. Discard transient inference buffers after metadata generation. Do not write a raster disparity track.
6. Load the new MOV with public `CNAssetInfo`. Require `needsPreprocessing`, resource status `ready`, and `isPreprocessed == false` before the runtime publishes the file.

The global rendering block uses a compatible virtual capture profile and rendering version 5. Hardware/sensor fields, the virtual sensor region, and the luma-derived scene-illuminance proxy are synthetic. They do not recover the source camera calibration, stabilization crop, or original focus choices. No native sample's Cinematic payload or disparity track is copied into the output.

The generator keeps the selected focal length constant. Optional external observations estimate effective field of view from sampled images; the native generator does not infer optical zoom or recover physical calibration. Original Cinematic geometry, disparity maps, and focus decisions are diagnostic references only. Subject continuity is an independent selection rule. It does not reproduce Apple's closed automatic director or guarantee the original camera's focus decisions.

The Rust CLI remains the product entry point. The Swift/Objective-C adapter performs Apple framework operations. The output is self-contained.

## Verification

Private samples can remain outside Git. Run the following command from this checkout. Use new result and build directories, or reuse existing build directories. The result directory must not exist.

```bash
python3 scripts/apple/verify_cinematic_envelope.py \
  --input /absolute/path/native.MOV \
  --run-dir /absolute/path/new-results \
  --swift-build-dir /absolute/path/swift-build \
  --rust-build-dir /absolute/path/rust-build \
  --synthetic --compare-render --verify-batch
```

`--verify-batch` also runs the supported stripped and synthetic inputs through directory discovery and parallel `batch --apple-cinematic`, then checks the MOV result kind, packet identity, and public processing/rendering/edit reload for each output. `--no-previews` omits regenerable PNG files while retaining numerical evidence. `--automatic-reference` removes native user decisions in memory, uses the resulting base-script focus for rendering, and records this reference mode. Repeat `--input` for more native samples. The runner builds the current CLI and adapter. It removes all non-video/non-audio tracks and global metadata from each native sample. Public `CNAssetInfo` must report `none` for this ordinary derivative. The runner then invokes the CLI to generate an independent pending envelope.

The public consumer check validates the script, recorded ID namespaces, selected detection track resolution, stable per-track detection type, finite frame focus, frame count, timestamps, aperture, timed keys, rendering frame attributes, virtual sensor geometry, per-frame focal-length range, and pending resource state. It adds two public fixed-focus tracks and requires their allocated IDs to be disjoint from recorded IDs. Pending resources must be preprocessed before editing. The check records base-decision counts and the focal-length timeline. FFprobe compares individual encoded video/audio packets, hashes, timestamps, and audio trim data. AVAssetReader audio buffer grouping is not a packet-identity test. The runner checks source hashes again after each operation. `--synthetic` adds independent 1080p and 360p H.264 inputs with no original Cinematic data or detectable faces. With `--compare-render`, the 1080p input must pass the full native processing checks. The 360p input is a separate compatibility probe: only Apple's explicit `unsupported` preprocessing error is recorded as a limitation; other errors fail the run. An unsupported compatibility probe never counts as rendering or editing evidence. If a later system supports it, the full rendering and edit checks run.

Import the pending MOV into a Photos test library. Enable Cinematic, allow Photos to process its resources, change the aperture and focus, then save and reopen the edit. Verify that the rendered effect changes. Public preprocessing and script reload tests do not replace this consumer check. Ordinary sources have no Cinematic audio metadata; the command preserves their audio samples, but it does not generate Apple's Cinematic audio mix effects.

## Headless effect comparison

`--compare-render` invokes public `CNAssetInfo` preprocessing separately for the native and regenerated asset. Both must become `renderable`. It then uses public `CNRenderingSession` at export quality to render frames at 25%, 50%, and 75% of the common duration, at f/2 and f/16. Both assets use the same decoded color, timestamps, 10-bit input format, output geometry, and identity display transform.

After preprocessing, the tool also adds two public custom detection tracks and focus decisions to the generated asset, changes aperture, serializes `CNScript.Changes`, and reloads them. It verifies disjoint track IDs, all saved detections/timestamps, finite frame focus, aperture, and focus at both edited decision times. It records changes to the intervening focus curve: the public loader recalculates transitions, including for native camera controls. It does not require those transitions to equal the immediate in-memory curve. The synthetic input follows the same preprocessing, export rendering, and edit/reload checks without a native reference. It does not require a measurable blur change for a flat scene. To check a generated file alone, run the renderer with `--verify-generated GENERATED.mov NEW_OUTPUT_DIRECTORY`.

Measurements use full-resolution `RGBA16Float` texture RGB values. The native RGBA destination retains the source transfer function. For HLG input, these values are HLG-encoded BT.2020 RGB. They are not linear sRGB. The tool also converts the rendered image to extended linear BT.2020 and records separate linear render and effect differences. Both measurements exclude alpha. The report also exports every processed script frame's detection IDs, groups, types, rectangles, focus values, base decision times, and transition ranges. Raw IDs belong to each asset and must not be compared as shared identities. If native and inferred depth raster dimensions differ, the tool validates each raster separately and records the depth difference as incomparable; it still renders each asset directly at the unchanged color-video resolution. The tool hashes decoded color planes and requires identical inputs. The report records exact pixel identity, mean absolute error, root mean square error, peak signal-to-noise ratio, and the fraction of components with an absolute difference above 0.001. It also records each focus rectangle. It compares inferred disparity and the f/2 versus f/16 effect across the full frame and within the native focus rectangle. Effect cosine similarity measures whether the blur changes occur in similar areas. A repeated native render measures determinism.

The generated render with a native focus value is a diagnostic control. Disparity scale and offset are undefined. A shared number does not prove that two different maps use the same physical focus plane.

To compare a new verification run with an earlier run, use:

```bash
python3 scripts/diagnostics/compare_cinematic_alignment.py \
  --baseline /absolute/path/baseline/report.json \
  --current /absolute/path/current/report.json \
  --output /absolute/path/current/alignment.json \
  --require-improvement SAMPLE_NAME --minimum-reduction 0.25
```

The tool requires the current report to match `HEAD`. It checks source hashes, ordinary-input integrity, media packet identity, decoded color identity, sample times, reference focus mode, render settings, native repeatability, and unchanged reference effect energy. The required sample must reduce both mean f/2 render error and mean relative effect error. The report also shows every other sample, including regressions. This check does not turn a measured improvement into an equivalence guarantee.

Earlier reports incorrectly labeled the unconverted texture values as `raw linear RGB`. Their encoded RGB measurements can be compared with current encoded RGB measurements only when decoded color hashes match and native reference effect energy is unchanged. The comparison tool records this label correction. Earlier PNG previews that interpreted HLG textures as linear sRGB do not provide a valid visual reference.

PNG files are SDR previews. Core Image uses the decoded buffer's color space, content headroom, and average light level. `CISystemToneMap` maps the image to standard dynamic range with display headroom 1. The tool converts the source display transform from video coordinates to Core Image coordinates. It writes 8-bit PNG with an sRGB profile. These previews are not floating point measurement data. They do not establish equality with Dolby Vision dynamic-trim playback in Photos. Test preprocessing derivatives reference their unchanged input color tracks to avoid duplicate data. These test derivatives are not portable; the product `pending.mov` remains self-contained.

Run the HDR color regression with the renderer built by the verification runner:

```bash
/absolute/path/new-results/cinematic-render-comparison \
  --verify-color /absolute/path/new-color-results
```

Known HLG patches must match a color-managed reference. The former linear-sRGB interpretation must fail the negative control. The SDR PNG must have an sRGB profile and distinct highlight levels after system tone mapping. Asymmetric corner markers must match independent Core Image orientation references at 0, 90, 180, and 270 degrees.

A completed comparison is not an equivalence result. Use `all_compared_frames_identical` and the numerical differences to assess equivalence. Missing camera calibration, optical geometry, or original focus decisions can produce differences even when both assets render successfully. This test does not prove Apple Photos save/reopen behavior, device Gallery behavior, or equality at unsampled frames.

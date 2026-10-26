# Pending Cinematic MOV

English | [简体中文](cinematic.md)

`xdremux convert --apple-cinematic` creates a new pending Cinematic MOV from ordinary video. The output must reach Apple's public `needsPreprocessing` state before publication. Inferred effects do not have a proven equivalence to the original camera effects.

## Command and requirements

```bash
xdremux convert --input ordinary.mp4 --output pending.mov --apple-cinematic
```

Use `--apple-cinematic` with `convert` or `batch`. It cannot be combined with image-feature flags or `--oppo-compatible`. The output must have a `.mov` extension and must not exist. For single-file conversion without an explicit output, the command creates `SOURCE.xdremux.mov` beside the source.

The [Rust runtime](../crates/xdremux-runtime/src/lib.rs) hashes the source before and after generation. It rejects a changed source. It publishes the candidate with a hard link that cannot replace an existing output.

The current implementation requires macOS 27 or later, a macOS 27 SDK build, and available Apple inference resources. It also requires the private Portrait/Cinematic methods used by the [native adapter](../Sources/XDCinematicNative/cinematic_envelope.m). Before each required private Objective-C call, the adapter checks the method's type encoding. An unsupported method, SDK, system, or resource causes failure. Older SDK builds retain other adapter operations but reject Cinematic generation before inference.

| Option | Default | Permitted range | Meaning |
| --- | --- | --- | --- |
| `--focal-length-35mm MM` | `24` | `1` through `300` | Focal-length assumption for every frame |
| `--aperture F` | `2.8` | `1` through `64` | Aperture assumption |

Both values must be finite. Either option requires `--apple-cinematic`. When camera geometry is absent, these values are capture assumptions. They are not measured optical properties.

### Recorded compatibility results

This command is experimental. `needsPreprocessing` and resource status `ready` confirm that Apple recognizes the metadata. They do not establish that later preprocessing supports every input.

Earlier project checks on macOS 27.0.1 recorded these results:

| Input | Result |
| --- | --- |
| 1080p/4K HEVC camera inputs with Cinematic data removed | Public preprocessing, rendering, and edit reload passed |
| Independently encoded 1080p H.264 | Public preprocessing, rendering, and edit reload passed |
| Matched 1920×1080, 30 fps short clips in H.264, HEVC, and ProRes Standard | All three codecs passed the public processing path |
| Independently encoded 640×360 H.264 | Pending generation passed; preprocessing returned `CNCinematicErrorDomain` code `5` (`unsupported`) |

These checks show that the tested public processing path does not require HEVC. Changing color tags or virtual sensor metadata did not resolve the 640×360 rejection. The command preserves compressed media. It does not resize or transcode the input to bypass that restriction. Other dimensions, codecs, systems, and iPhone Photos behavior require separate consumer checks.

## Information required for processing

The inference path first decodes the video. A supported codec alone is insufficient: the file also needs a valid Cinematic metadata envelope. The tested public path uses the following information:

| Information | Requirement |
| --- | --- |
| Global `com.apple.quicktime.cinematic-video` block | Describes the Cinematic asset |
| Per-frame cinematography | Contains valid detection IDs and focus decisions |
| Per-frame rendering | Contains camera-geometry assumptions |
| Video and metadata timing | Describes the same visible timeline |

A raster disparity track is not required for `needsPreprocessing`.

### Diagnostic tests with metadata keys disabled

A diagnostic test renamed keys without changing media bytes or container offsets. Disabling cinematography changed capability to `none`. Disabling the global block left a pending asset whose resources were not ready. Disabling timed rendering left a ready pending asset, but processing failed. Thus, pending classification does not validate all processing inputs.

Another test disabled several keys together in the 1080p H.264 short clip. The keys described recognized intent, `metadata.1`, aperture/disparity scalars, and scene illuminance. Public preprocessing, rendering, and edit reload still passed. The retained rendering and script blocks contained their own values. An autofocus-only script with one initial decision also passed.

Production output retains these fields. These tests do not establish the minimum schema that Photos requires. A codec does not recover physical autofocus, original camera calibration, or missing depth.

## Native generation path

The Rust CLI is the product entry point. The Swift/Objective-C adapter performs Apple framework operations. The resulting MOV is self-contained.

### Decode and select subject seeds

AVFoundation decodes the selected color-video track. `PTMonocularDisparityProvider` uses each decoded frame and the assumed focal length to infer temporary disparity. Vision receives the video's display orientation. Detected rectangles are then mapped back to encoded coordinates.

Subject selection follows this order:

1. Use detected faces.
2. If no face is present, detect upper bodies and body-pose landmarks.
3. If no human seed remains, use animal recognition and dog/cat head landmarks.
4. If no subject seed remains, use objectness saliency.

| Seed | Conditions and geometry |
| --- | --- |
| Human head from body pose | Both ears need confidence of at least `0.3`. Use their midpoint and `1.4` times their horizontal separation. The normalized width must be `0.01` through `0.5`. Use the oriented image aspect ratio to represent a square display-pixel region. Prefer up to two head seeds; otherwise use upper-body seeds. |
| Animal head | The recognized dog/cat label needs confidence of at least `0.5`. Match head landmarks to the recognized body. A nose and at least one eye need confidence of at least `0.3`. Require at least three confident head points and at least `60%` head overlap with the body. Otherwise use the body seed. |
| Objectness saliency | Choose the region with the highest confidence. Require confidence of at least `0.3` and area from `0.5%` through `60%` of the image. Use detection type `unknown`; this does not identify a person. |

### Track subjects and create focus decisions

Public `CNObjectTracker` continues up to two subjects while bounds and confidence remain valid. Tracking can continue through detector gaps. A new subject receives a new track and group. Each track keeps its detection type.

Animal matching uses both head and body regions. It keeps dog/cat identities separate from generic saliency. A tracked animal head survives missing pose detections while the tracker remains valid. When an animal part returns, the generator reuses that part's track ID within its actor group. A human type reseed starts a new track. Matched human or animal head detections correct tracker drift.

The selection rule first favors a human over a non-human seed. A detected head can then take focus if its area exceeds the selected rectangle's area by more than `1/0.6`. Otherwise, a valid selected subject remains selected. If no subject remains, the generator selects a center autofocus region.

The autofocus track is separate from subject tracks. Recorded track/focus allocator namespaces leave public editor namespaces available. Each subject change creates a base decision. `PTCinematographyPostcaptureRefinement` creates and serializes the script. During a transition, the refined focus detection agrees with the frame header. The base selection is stored separately.

### Write metadata and publish the MOV

The generator writes new global Cinematic metadata and one timed metadata group per decoded frame. It uses these values:

| Field | Value |
| --- | --- |
| Unknown capture focus/disparity | Native pending sentinel `-1` |
| Visible crop | `(1,1)`, so Apple's parser retains the per-frame assumed focal length |
| Virtual sensor region | Encoded image size |
| Aperture | Written to both the script and timed rendering data |
| Global rendering profile | Compatible virtual capture profile, rendering version `5` |

The generator copies compressed video and all supported audio tracks. It preserves the video transform and track timescale. It discards temporary inference buffers after metadata generation. It does not write a raster disparity track or copy Cinematic payloads from a native sample.

Before publication, public `CNAssetInfo` must accept all three conditions: capability `needsPreprocessing`, resource status `ready`, and `isPreprocessed == false`.

### Limits of inferred capture data

Hardware/sensor fields and the virtual sensor region are synthetic. Scene illuminance is a proxy derived from luma. These values do not recover camera calibration, stabilization crop, or original focus choices.

The supplied focal-length assumption stays constant. The generator does not infer optical zoom or absolute field of view from image motion. Original Cinematic geometry, disparity maps, and focus decisions are diagnostic references only. Subject continuity uses independent selection rules. It does not reproduce Apple's closed automatic director or guarantee the original focus decisions.

## Verification

Private samples can remain outside Git. From the checkout, run the following command. The result directory must not exist. Build directories can be new or reused.

```bash
python3 scripts/apple/verify_cinematic_envelope.py \
  --input /absolute/path/native.MOV \
  --run-dir /absolute/path/new-results \
  --swift-build-dir /absolute/path/swift-build \
  --rust-build-dir /absolute/path/rust-build \
  --synthetic --compare-render --verify-batch
```

The [verification runner](../scripts/apple/verify_cinematic_envelope.py) builds the current CLI and adapter. For each native sample, it removes global metadata and all tracks other than video and audio. Public `CNAssetInfo` must classify this ordinary derivative as `none`. The runner then invokes the CLI to generate an independent pending envelope.

| Option | Additional behavior |
| --- | --- |
| Repeated `--input` | Adds native samples |
| `--verify-batch` | Tests directory discovery and parallel `batch --apple-cinematic` with supported stripped and synthetic inputs. Checks MOV result kind, packet identity, public preprocessing, rendering, and edit reload for each output. |
| `--no-previews` | Omits PNG files that can be regenerated; retains numerical evidence |
| `--automatic-reference` | Removes native user decisions in memory. Renders with the resulting base-script focus and records that reference mode. |
| `--synthetic` | Adds independent 1080p and 360p H.264 inputs without original Cinematic data or detectable faces |

### Public consumer and media checks

The public consumer check validates the following groups:

| Group | Checked information |
| --- | --- |
| Script identity | Recorded ID namespaces, selected detection track resolution, stable detection type per track |
| Frame and timing data | Finite frame focus, frame count, timestamps, base-decision counts, focal-length timeline |
| Rendering metadata | Aperture, timed keys, rendering frame attributes, virtual sensor geometry, per-frame focal-length range |
| Resource state | Pending resource state; preprocessing before editing |
| Public editing | Two added fixed-focus tracks with IDs disjoint from recorded IDs |

FFprobe compares individual encoded video/audio packets, hashes, timestamps, and audio trim data. AVAssetReader audio buffer grouping does not establish packet identity. The runner rechecks source hashes after each operation.

With `--compare-render`, the synthetic 1080p input must pass the full native processing checks. The 360p input is a separate compatibility probe. Only Apple's explicit `unsupported` preprocessing error is recorded as a limitation; other errors fail the run. An unsupported probe supplies no rendering or editing evidence. If a later system supports it, the full rendering and edit checks run.

### Photos consumer check

1. Import the pending MOV into a Photos test library.
2. Enable Cinematic.
3. Wait for Photos to process the resources.
4. Change the aperture and focus.
5. Save the edit, then reopen it.
6. Verify that the rendered effect changes.

Public preprocessing and script reload do not replace this check. Ordinary sources have no Cinematic audio metadata. The command preserves their audio samples but does not generate Apple's Cinematic audio mix effects.

## Headless effect comparison

`--compare-render` separately preprocesses the native and regenerated asset through public `CNAssetInfo`. Both must become `renderable`. Public `CNRenderingSession` then renders at export quality. It samples `25%`, `50%`, and `75%` of the common duration, at f/2 and f/16. Both assets use identical decoded color, timestamps, 10-bit input format, output geometry, and an identity display transform.

### Editing and reloading

After preprocessing, the tool adds two public custom detection tracks and focus decisions to the generated asset. It changes aperture, serializes `CNScript.Changes`, and reloads them. The checks cover disjoint track IDs, saved detections and timestamps, finite frame focus, aperture, and focus at both edited decision times.

The public loader recalculates transitions, including those in native camera controls. The tool records differences in the intervening focus curve. It does not require those transitions to equal the immediate in-memory curve.

Synthetic inputs receive the same preprocessing, export rendering, and edit/reload checks without a native reference. A flat scene does not need a measurable blur change. To check a generated file alone, run the renderer with `--verify-generated GENERATED.mov NEW_OUTPUT_DIRECTORY`.

### Measurement space and metrics

Measurements use full-resolution `RGBA16Float` texture RGB values and exclude alpha. The native RGBA destination retains the source transfer function. For HLG input, the values are HLG-encoded BT.2020 RGB. The tool separately converts the rendered image to extended linear BT.2020. It reports linear render and effect differences separately from the source-transfer measurements.

Decoded color-plane hashes must be identical. If native and inferred depth raster sizes differ, the tool validates each raster separately and marks depth differences as incomparable. It still renders each asset at the unchanged color-video resolution.

| Evidence | Recorded values |
| --- | --- |
| Render differences | Exact pixel identity, mean absolute error, root mean square error, peak signal-to-noise ratio, and fraction of components with absolute difference above `0.001` |
| Depth and blur effect | Inferred disparity and the f/2 versus f/16 effect, over the full frame and within the native focus rectangle |
| Effect location | Effect cosine similarity, which measures whether blur changes occur in similar areas |
| Repeatability | Repeated native rendering |
| Complete processed script | Every frame's detection IDs, groups, types, rectangles, focus values, base-decision times, and transition ranges |

Raw detection IDs belong to each asset. They do not identify shared subjects across assets. A generated render using the native focus value is a diagnostic control. Disparity scale and offset are undefined. Equal numbers do not establish an equal physical focus plane in different maps.

### Compare verification runs

Use the [alignment comparison tool](../scripts/diagnostics/compare_cinematic_alignment.py) to compare a new run with an earlier run:

```bash
python3 scripts/diagnostics/compare_cinematic_alignment.py \
  --baseline /absolute/path/baseline/report.json \
  --current /absolute/path/current/report.json \
  --output /absolute/path/current/alignment.json \
  --require-improvement SAMPLE_NAME --minimum-reduction 0.25
```

The current report must match `HEAD`, and the output must not exist. The tool checks source hashes, ordinary-input integrity, packet identity, decoded color identity, sample times, and reference focus mode. It also checks render settings, native repeatability, and unchanged reference effect energy.

For `SAMPLE_NAME`, both mean f/2 render error and mean relative effect error must fall by at least `25%`. The report includes all other samples, including regressions. Measured improvement does not establish equivalence.

Earlier reports incorrectly labeled unconverted texture values as `raw linear RGB`. Comparison with current encoded RGB measurements requires matching decoded color hashes and unchanged native reference effect energy. The tool records this label correction. Earlier PNG previews that treated HLG textures as linear sRGB are invalid visual references.

### SDR previews and HDR color regression

PNG files are SDR previews. Core Image uses the decoded buffer's color space, content headroom, and average light level. `CISystemToneMap` maps the image to standard dynamic range with display headroom `1`. The tool converts the source display transform from video coordinates to Core Image coordinates. It writes 8-bit PNG files with an sRGB profile.

These previews contain no floating-point measurement data. They do not establish equality with Dolby Vision dynamic-trim playback in Photos. Test preprocessing derivatives reference unchanged input color tracks to avoid duplicate data. These derivatives are not portable. The product `pending.mov` remains self-contained.

With the renderer built by the verification runner, run the HDR color regression:

```bash
/absolute/path/new-results/cinematic-render-comparison \
  --verify-color /absolute/path/new-color-results
```

Known HLG patches must match a color-managed reference. The former linear-sRGB interpretation must fail the negative control. The SDR PNG must contain an sRGB profile and distinct highlight levels after system tone mapping. Asymmetric corner markers must match independent Core Image references at `0`, `90`, `180`, and `270` degrees.

### Interpret the result

A completed comparison reports that the checks ran. Use `all_compared_frames_identical` and the numerical differences to assess equality. Missing camera calibration, optical geometry, or original focus decisions can cause differences even when both assets render.

These tests do not establish Photos save/reopen behavior, device Gallery behavior, or equality at unsampled frames.

# Video Smart Style carrier experiment

English | [简体中文](README.zh-CN.md)

This experiment compares four candidate MOV representations for video Smart Style metadata. **It is research only.** It does not establish a supported XDRemux conversion or reversible video editing in Apple Photos.

Rust owns product policy and orchestration. The isolated Swift executable in this directory performs public AVFoundation operations for the experiment.

## Candidate definitions

All four candidates write six static `com.apple.quicktime.smartstyle.*` fields. Three candidates also write a boxed timed metadata track with binary-plist payloads.

| Variant | Timed metadata |
| --- | --- |
| `static` | None; this is the control without a timed track. |
| `lower` | Lower-case initial letters in the longer field names. |
| `compact` | Short field names. |
| `upper` | Upper-case initial letters in the longer field names. |

[`CarrierProbe.swift`](CarrierProbe.swift) defines all variants in one constructor. The constants are rendering version/cast 1, intensity 1, tone/color 0, bypass false, and reversible true where applicable. These are synthetic hypotheses. They are not recovered scene facts or a validated native schema.

## Run the experiment

Use macOS with Xcode command-line tools and `ffprobe`. The synthetic smoke test also requires `ffmpeg`. Run these commands from the repository root:

```sh
swiftc -swift-version 5 -parse-as-library research/video_styles/CarrierProbe.swift -o /tmp/CarrierProbe
python -m research.video_styles.probe input.mov candidate.mov --helper /tmp/CarrierProbe --variant lower
python -m research.video_styles.smoke --helper /tmp/CarrierProbe
```

Use the Python wrapper to publish a validated candidate. The Swift constructor alone writes to a private output path and does not perform the complete publication checks.

The destination must not exist, and its parent directory must exist. Use only media that you have permission to process. The wrapper does not upload media.

| Limit | Value |
| --- | --- |
| Input file size | At most 2 GiB |
| Subprocess deadline | 120 seconds |
| Swift reader/writer loop deadline | 100 seconds |
| Video samples accepted by the timestamp reader | At most 100,000 |

These limits bound diagnostic work. They are not product performance targets.

## Checks before publication

[`probe.py`](probe.py) constructs the candidate in a private temporary folder in the destination directory. It requires all of the following checks before publication:

- Native readback of the static and timed metadata payloads.
- SHA-256 equality for every encoded audio/video packet, with matching per-track packet order and counts.
- Exact rational equality for PTS, DTS, and durations.
- Matching codec configuration, bit depth, chroma, color information, and display matrix.
- Matching source tags reported by `ffprobe`, except the documented container and stream bookkeeping fields.
- An unchanged source file hash.

The tag comparison excludes container brand/encoder fields and stream handler/vendor/encoder fields. These checks do not establish preservation of uninterpreted MOV atoms.

The wrapper publishes the completed candidate through an atomic hard link. If another writer creates the destination first, this process refuses to replace it. Atomic name publication does not guarantee survival after a filesystem or power failure.

The experiment accepts one video track and optional audio tracks. It rejects chapters, other track types, missing or overlapping timing, changing codec descriptions, and pre-existing Smart Style keys. Parity failures and unsupported filesystems also fail explicitly.

The tool passes through encoded media. It does not re-encode media, invent frame durations, replace the input, or convert native errors and timeouts into success.

## Native timing observation

The following result belongs to the recorded macOS 26.6.2 (25G83) experiment. It is not a new run or evidence about iOS 27 Photos.

Compressed `AVAssetReader` emitted zero-length `EmptyMedia` control buffers at the track presentation end. Forwarding those markers to `AVAssetWriter` changed the last H.264 B-frame packet duration from 1/12 to 1/4 second. Packet payloads and track presentation duration remained equal. An explicit `endSession` call did not correct the change.

Consuming only the terminal empty markers preserved all packets and timing in the tested H.264 and HEVC10/audio/rotation matrix. Other control buffers and real samples, including audio trim attachments, remain unchanged. Interior empty edits fail explicitly. The full-packet comparison remains a publication requirement.

The synthetic smoke test requires the terminal-marker path to run. This check covers the tested public-framework behavior only.

## Failure reports and regression tests

The probe writes a JSON report and returns a nonzero exit status on failure. A timing mismatch records:

- Media type, source/candidate track IDs, and stream indices.
- Packet index, or null for a stream-level timing mismatch.
- Raw timestamp ticks, time bases, and exact rational values.

The tool hashes a temporary candidate before cleanup. It does not publish a candidate after a parity failure or retain that failed candidate as an artifact. Source readback distinguishes unchanged, changed, and unavailable input. Unavailable evidence is null.

The synthetic gate covers H.264/HEVC10 × `static`/`lower`/`compact`/`upper`: eight cases in total. It records every case even if an earlier case fails. All eight cases must pass, and the terminal empty-marker path must run. The gate also requires no-clobber, source-immutability, and malformed-input controls.

A missing prerequisite cannot satisfy the malformed-input control. CI uploads JSON diagnostics after success and failure, but upload success does not override a failing test. If a carrier trial publishes successfully and a later control fails, the overall test fails. The report still records the earlier publication.

Run the portable regressions with:

```sh
python -m unittest tests.test_video_style_research tests.test_video_style_diagnostics -v
```

The native structural gate is in [the research workflow](../../.github/workflows/research.yml). It generates H.264 and HEVC 10-bit clips with audio and rotation. These synthetic clips are not native Smart Style positive samples. Passing research CI does not change product support.

## Historical experiments and rejected methods

Earlier jobs failed during source download/LFS handling and Swift bridging or concurrency checks. Their validator read only the first video sample. It could not establish complete media preservation, metadata readback, Photos admission, or edit/save/reopen behavior.

The earlier writer deleted an existing destination before validating its input. It also assumed 30 fps when duration information was missing. Both methods are retired.

A removed firmware-analysis workflow requested iOS 27 build `24A437`. Its targets were:

- `PESupport.assetCanRenderStyles:`
- `PESupport.canPerformEditOnAsset:`
- `PISemanticStyleAutoCalculator.canRenderStylesOnComposition:`
- `PISemanticStyleAutoCalculator.isStylableFromImageProperties:error:`
- `PHAssetPhotosSmartStyleExtendedProperties.isCurrentlySmartStyleable`
- `PHAsset._setupSmartStyleFromFetchDictionary:`

These names identify probe targets, not successful observations. The disassembly output was not committed, and the workflow allowed some failures to exit successfully. CameraUI addresses `0x1bdd478fc` and `0x1bdf25490` applied to a specific build; they are not portable entry points. This directory does not restore the firmware downloader, private binaries, or one-shot firmware workflows.

## Requirements before product use

Product use requires a legally shareable native positive/control corpus with recorded OS and device provenance. Each sample needs correct metadata semantics. Repeated static values do not establish those semantics.

Validation must cover complete duration, audio, color/HDR, and orientation preservation. On applicable devices, it must also cover Photos import, independent parameter editing, saving, reopening, and reversal. Reject a candidate when a control or negative test disproves it.

Preserve raw consumer observations and failures with source/output hashes. Until these requirements pass, `photosEditingValidated` remains false regardless of CI status.

## Primary references

- [AVFoundation export guide](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/AVFoundationPG/Articles/05_Export.html): nil output settings request compressed passthrough. Check reader/writer completion and errors.
- [Metadata adaptor](https://developer.apple.com/documentation/avfoundation/avassetwriterinputmetadataadaptor): timed groups need an explicit boxed format description. Check behavior against the installed SDK and native readback.
- [ffprobe](https://ffmpeg.org/ffprobe.html): `-show_packets -show_streams -show_data_hash sha256` provides packet and codec-extradata checksums. Rational timestamps avoid comparisons based on rounded values.

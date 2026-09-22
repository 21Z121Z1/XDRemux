# Video Smart Style carrier hypothesis

English | [简体中文](README.zh-CN.md)

**Research only.** This tests four candidate MOV representations, not a supported
XDRemux conversion and not evidence of reversible Apple Photos video editing.
Rust remains the sole product policy/orchestration stack. The Swift executable
here owns only public AVFoundation operations for this isolated experiment.

## Retained question and observations

The historical experiment tried six static `com.apple.quicktime.smartstyle.*`
fields and three binary-plist spelling variants in a boxed timed metadata track:
`lower`, `compact`, `upper`. `static` is the no-timed-track control. The constants
(rendering version/cast 1, intensity 1, tone/color 0, bypass false, reversible true)
are **synthetic hypotheses**, not recovered scene facts or a validated schema.
The spelling alternatives live in one constructor, not competing backends.

Historical jobs encountered source download/LFS and Swift bridging/concurrency
failures. The old validator read only the first video sample. It did not prove
full audio/video preservation, metadata payload readback, Photos admission, or
edit/save/reopen. The old writer also deleted an existing output before checking
its source and guessed 30 fps for missing durations. Those approaches are retired.

The removed firmware-analysis workflow requested iOS 27 build `24A437` and probed
`PESupport.assetCanRenderStyles:`, `PESupport.canPerformEditOnAsset:`,
`PISemanticStyleAutoCalculator.canRenderStylesOnComposition:`,
`PISemanticStyleAutoCalculator.isStylableFromImageProperties:error:`,
`PHAssetPhotosSmartStyleExtendedProperties.isCurrentlySmartStyleable`, and
`PHAsset._setupSmartStyleFromFetchDictionary:`. These are **probe targets**, not
successful observations: its disassembly output was not committed and failures
were allowed to exit successfully. Two build-specific CameraUI addresses
(`0x1bdd478fc`, `0x1bdf25490`) are not portable entry points. No firmware downloader,
private binary, or one-off firmware Actions workflow is revived here.

## Reproducible carrier experiment

On macOS, with Xcode command-line tools and `ffprobe` installed, from repository root:

```sh
swiftc -swift-version 5 -parse-as-library research/video_styles/CarrierProbe.swift -o /tmp/CarrierProbe
python -m research.video_styles.probe input.mov candidate.mov --helper /tmp/CarrierProbe --variant lower
python -m research.video_styles.smoke --helper /tmp/CarrierProbe
```

Use only media you may process. The wrapper never uploads media. CI generates
its own H.264/HEVC 10-bit clips with audio and a rotation matrix; these are not
native Smart Style positive samples. The input limit is 2 GiB; subprocesses have
120-second deadlines; the timestamp reader accepts at most 100,000 video samples.
These bounds keep a diagnostic run finite, not define product performance goals.

`probe.py` requires an absent destination and an existing parent directory. It
constructs inside a private same-directory temporary folder. Before atomically
linking the finished candidate into place it requires native static/timed payload
readback and complete encoded-packet SHA-256, per-track order/count, exact rational
PTS/DTS/duration, codec configuration, bit depth/chroma/color and display-matrix
parity. It checks source tags reported by ffprobe except container-brand/encoder
and stream handler/vendor/encoder bookkeeping; it does not claim preservation of
uninterpreted MOV atoms. The original input hash must remain unchanged. A racing
writer wins its own destination; this process refuses to replace it. Atomic name
publication is not a promise of survival after a filesystem/power failure.

Only one video plus optional audio tracks are currently supported. Chapters,
other source tracks, missing/overlapping timing, changing codec descriptions,
pre-existing Smart Style keys, parity failures and unsupported filesystems fail
explicitly. There is no re-encoding, silent fallback, invented frame duration, or
input replacement. Call the Python wrapper rather than the private-output Swift
constructor for validated publication. Native errors and subprocess timeouts are
reported, not converted into success.

## Promotion gate

A structurally readable file is insufficient. Promotion additionally requires a
legally shareable native positive/control corpus with OS/device provenance;
correct metadata semantics per sample rather than static repeated guesses;
full-duration, audio, color/HDR and orientation preservation; Photos import and
actual independent parameter edit, save, reopen and reversal on supported devices.
Reject a hypothesis when the control/negative matrix disproves it. Preserve raw
consumer observations and failures with source/output hashes. Until those gates
pass, `photosEditingValidated` remains false regardless of CI results.

## Design references

- [AVFoundation export guide](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/AVFoundationPG/Articles/05_Export.html): nil output settings request compressed passthrough; reader/writer completion and errors must be checked.
- [Metadata adaptor](https://developer.apple.com/documentation/avfoundation/avassetwriterinputmetadataadaptor): append timed metadata groups with an explicit boxed format description; the installed SDK and native readback are the executable contract.
- [ffprobe](https://ffmpeg.org/ffprobe.html): `-show_packets -show_streams -show_data_hash sha256` supplies complete packet and codec-extradata checksums. Rational timestamps avoid rounding-based false equivalence.

Portable regressions: `python -m unittest Tests.test_video_style_research -v`.
Native structural gate: `.github/workflows/research.yml`, separate from product
acceptance. A passing research gate never changes the support status above.

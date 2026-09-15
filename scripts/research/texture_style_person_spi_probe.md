# Texture Style person SPI probe

This is an isolated research probe for macOS 27. It does not change XDRemux production code or HEIF output.

It tests two hypotheses from the iOS/macOS 27 CMImaging and PhotoImaging diffs:

1. `+[CMITextureStylesPersonInputDataUtilities personInputDataArrayFromDetectedFaces:]` can canonicalize a synthetic FigCapture-style face dictionary into `CMITextureStylesPersonInputData`, including `faceROI`, `faceSkinROI`, landmark scaling ROI, and pose fields.
2. `CMITextureStylesProcessor` treats `inputPersonData` and `inputSkinSmoothingFaceDetections` as distinct contracts in the real PhotoImaging path.

## Build and black-box matrix

Run on an Apple-silicon Mac with macOS 27:

```bash
scripts/research/run_texture_style_person_spi_probe.sh --all
```

Each candidate dictionary shape runs in a separate process so an Apple assertion/abort in one case does not erase the rest of the matrix. Results are written under `/tmp/xdremux-texture-style-person-probe-*` unless `XDREMUX_TEXTURE_PROBE_OUT` is set.

To exercise crop normalization too:

```bash
scripts/research/run_texture_style_person_spi_probe.sh \
  --all --normalize 0.03 0.04 0.94 0.92
```

The important outputs are `cases/*.json`, `cases/*.stderr.log`, and `status.tsv`.

## Observe the real PhotoImaging contract

The build script also emits `libXDRemuxTextureStyleTrace.dylib`. Inject it only into a local research harness that already exercises the Texture Style PhotoImaging/CMImaging path:

```bash
XDREMUX_TEXTURE_TRACE=1 \
XDREMUX_TEXTURE_TRACE_FILE=/tmp/texture-style-trace.jsonl \
DYLD_INSERT_LIBRARIES=/path/to/libXDRemuxTextureStyleTrace.dylib \
/path/to/local/harness <existing arguments>
```

The dylib forwards every call to the original implementation and only records these methods when they exist:

- `+[CMITextureStylesPersonInputDataUtilities personInputDataArrayFromDetectedFaces:]`
- `-[PITextureStyleProcessorKernel personInputDataFromStillProperties:]`
- `-[CMITextureStylesProcessor setInputPersonData:]`
- `-[CMITextureStylesProcessor setInputSkinSmoothingFaceDetections:]`

Do not disable SIP or system security for this probe. If a hardened executable rejects `DYLD_INSERT_LIBRARIES`, rebuild the local research harness without hardened runtime/library validation or make that harness `dlopen` the trace dylib before loading PhotoImaging.

## Interpretation

A useful positive result is not "the synthetic values equal the historical iPhone capture metadata." The target is an Apple-compatible post-hoc replay contract: determine which fields Apple derives deterministically from image-reconstructible face geometry and which inputs remain separate processor contracts.

Do not promote any ROI rule into production until the native trace and the synthetic input matrix agree on the relevant field semantics.

# Apple editing features

English | [简体中文](apple-features.md)

XDRemux exposes Apple editing features through the Rust `xdremux` CLI. Rust owns product policy and file publication. The Swift/Objective-C adapter performs the required Apple framework operations.

This guide describes Photographic Styles, Texture Styles, and Apple Portrait. The experimental video command has a separate [Cinematic guide](cinematic.en.md).

## Availability and commands

The complete product, App, and Apple adapter require Apple silicon (`arm64`) and macOS 27 or newer. The adapter has no Intel build. It checks the host before it reports capabilities or executes an operation.

The portable Rust core retains Linux, Windows, and macOS tests. These checks do not establish Apple feature availability on those hosts. A supported host still needs the required APIs, source resources, GPU results, and final-file validation.

| `convert` or `batch` option | Requested result |
| --- | --- |
| `--apple-styles` | Photographic Styles and inferred Texture Styles for a ProXDR still. |
| `--apple-portrait` | Apple Portrait with the complete semantic resource set. |
| `--apple-portrait-oppo` | Apple Portrait from OPPO depth and available source masks, without Vision. |
| `--apple-cinematic` | Experimental pending Cinematic metadata for ordinary video. See the [Cinematic guide](cinematic.en.md). |

These options are mutually exclusive. They also conflict with `--oppo-compatible`. They do not create separate Apple CLI subcommands. Low-level adapter, donor, backend, and solver controls are not public product options. The [CLI parser](../crates/xdremux-cli/src/lib.rs) defines the current combinations.

Standard HDR, OPPO-compatible output, Motion Photo conversion, batching, categorization, inspection, and portable validation remain Rust product operations.

## Platform boundary

| Layer | Responsibility |
| --- | --- |
| Rust CLI | Express the requested product operation. |
| Rust engine | Define resource models, compatibility rules, and validation policy. |
| Rust runtime | Run operations, assemble resources, validate output, and publish files. |
| Apple adapter | Call ImageIO, Core Image, Vision, Core ML, AVFoundation, VideoToolbox, and other required frameworks. |

`xdremux-apple-adapter` is a distributable component used by Rust. It is not a public conversion CLI.

The runtime communicates with the adapter through a versioned helper-process protocol with bounded messages and operation timeouts. Each conversion reuses one session that starts on demand. A timeout, crash, or framing error terminates that session. Transport remains private to the runtime.

ImageIO auxiliary probing reports concrete observations. These include ISO Gain Map, disparity, Portrait Effects Matte, and skin, hair, teeth, and glasses masks. The adapter does not decide whether the complete product result is valid. `xdremux-engine` makes that decision through `AppleImageAuxiliaryFacts` and the resource contract.

New adapter operations must return the framework facts or results that Rust needs. Product decisions remain in Rust. See [apple_adapter.rs](../crates/xdremux-runtime/src/apple_adapter.rs) and [apple.rs](../crates/xdremux-engine/src/apple.rs).

## Photographic Styles

Rust owns source-dependent style generation, constrained search, key 1 and plist synthesis, resource graphs, validation, and publication. The adapter supplies framework observations and encoding operations.

Research producers, model experiments, donor diagnostics, and RAW experiments remain research tools. They do not define another runtime or public conversion path.

### Texture Styles from the same switch

`--apple-styles` and the App's Photographic Styles switch generate Texture Styles as part of the same operation. There is no separate Texture option. Before reusing an output, the App runs `validate --apple-styles`. A plain HDR output does not satisfy that check.

Texture generation needs the private Vision and CMImaging APIs tested on macOS 27. A missing API or failed native operation rejects the entire Styles transaction before publication.

The analysis and output have different roles:

- ImageIO produces an upright sRGB analysis image with a maximum dimension of 1024 pixels.
- The output retains the full primary image, HDR Gain Map, Styles Linear Thumbnail, and existing item payloads.
- Analysis cannot recover RAW data or Apple capture-time processing.

Vision provides face landmarks, signed pose angles, person instances, and semantic masks. Maximum total mask overlap matches each measured face to a distinct person instance. Rust requests Mattify, UnderEyeBrighten, and SkinSmoothStandalone statistics for each matched person. All effects use the same inferred geometry and masks.

The native operation order is `setup`, `prewarm`, `prepareToProcess:`, `calculateStats`, `process`, GPU fence, and final `finishProcessing`. Analysis, inference, statistics, and final readback use separate typed requests and results. The Rust runtime's [adapter client](../crates/xdremux-runtime/src/apple_adapter/texture.rs) owns native report decoding and workfile names.

A helper acknowledgement alone does not prove statistics generation or permit publication. GPU completion, lifecycle, input binding, and final-byte readback checks still apply.

Rust combines only the statistics returned by the native operations. It omits successful but empty optional groups. A scene without matched faces gets an empty people array, not a template person. Nose, lip, eyebrow, and hand protection use regions derived from native landmarks or hand poses. Ear and tattoo protection are disabled for this derived edit. This does not reproduce the complete camera FSINC producer.

The Rust HEIF writer adds a version-3 Texture plist, separately encoded masks, and exact per-instance XMP reference keys. Its tuning profile selects a derived edit. It does not replace EXIF camera identity, exposure, lens, date, or capture UUID.

After relocation, the writer checks every original item payload. Before atomic publication, Apple's parser must read back the same person count, reference keys, geometry, and statistics.

On macOS 27, run the fixture check:

```bash
python3 scripts/apple/check_texture_styles.py --all-fixtures --output /absolute/path/new-texture-results
```

This checks real ProXDR conversion, batch conversion, unchanged primary and Gain Map HEVC, and preservation of existing output after failure. Earlier SDR research candidates have user-reported Photos activation. That feedback does not certify every full-resolution HDR product output, effect response, save, or reopen.

The implementation is in [apple_texture.rs](../crates/xdremux-runtime/src/apple_texture.rs) and [XDTextureNative](../Sources/XDTextureNative).

### SmartStyle edit state

The Standard Styles resource set also needs SmartStyle state in Apple MakerNote tag 84 (`0x0054`). The coefficient plist is a separate input. Successful plist parsing does not establish the photo's edit state.

Rust defines the required integer edit fields, including Standard Texture state. ImageIO merges those fields into the existing MakerApple dictionary. It does not add camera identity or a UUID. Other entries and unmodified cells remain intact.

Before publication, Rust stages the complete assembled file and asks ImageIO to read its integer dictionary cells. Missing or changed required fields reject the conversion before an existing output is replaced. An older adapter that ignores the added metadata request cannot silently publish an incomplete Styles file.

Native readback does not establish Photos controls, Tone/Color response, or save/reopen behavior. Those claims require device evidence.

### HDR reference for the Linear Thumbnail

The Styles Linear Thumbnail stores Apple Log codes for scene-linear Display P3 HDR reconstructed from the SDR base and Gain Map. It does not store a copy of the SDR image codes.

Rust applies the Gain Map at the base image's original resolution. It resamples map codes bilinearly before HDR reconstruction. Reconstruction uses the source's ISO 21496-1 or Apple `tmap` parameters:

- The minimum and maximum log2 gain.
- Gamma as the reciprocal exponent used by the 26A434 `_flexMap` decoder.
- Per-channel offsets.
- The alternate-image headroom weight.

After reconstruction, an area box filter reduces the image in linear light to the 1024-pixel-class thumbnail. Rust divides each channel by `baseline_exposure`, stored in Styles plist key `4`, then applies Apple Log.

The Apple Log transfer, `[0, 1]` code range, and auxiliary color signaling remain unchanged. The encoder inverts the macOS 26A434 exponential decoder, `_appleLogToLinear` / `sw_appleLogToLinear`. That decoder does not use a quadratic toe, so this encoder does not use one. The camera-side encoder toe remains unverified.

The transform keeps Display P3 primaries. It does not convert the thumbnail to the BT.2020 color space used by Apple Log video. It also does not relabel the thumbnail as sRGB, PQ, HLG, or linear.

Coefficients use the decoded image in display orientation. Before RGB16 encoding, Rust reverses the primary item's complete rotation and mirror sequence. Stored Linear Thumbnail and Style Delta Map items use the primary coded geometry. They preserve the relative order of `irot` and `imir`.

A Gain Map or Styles item with an incompatible aspect ratio is rejected. `encoding_gain` is renderer metadata, not another exposure divisor.

The codec receives 16-bit Apple Log codes. VideoToolbox converts them to 10-bit video-range 4:2:0 and encodes Main10 HEVC. The input does not pass through RGB8 quantization, so the encoder can use the 10-bit range. HDR values above SDR white remain represented.

This reference derives from the rendered HDR appearance. It cannot recover RAW data or capture-time processing already lost from the source.

### Styles coefficient fields

The 26A434 `PISemanticStyleFilter` consumer determines how XDRemux constructs these fields.

| Field | Stored form | Construction and limit |
| --- | --- | --- |
| Key `3` | One 256-node curve, sampled independently for each channel. | The x value is `min(S, 1)` after the plist base gain. There is no luminance reduction or percentile normalization. |
| Keys `c` and `d` | Two 32×32 guide maps. | Each cell stores an area mean. `c` represents the primary image. `d` represents the linear thumbnail. |
| Key `1` | An x-major 12×9×8×10×3 half array. | Each spotlight cell and luminance plane is fit separately with a quadratic basis. |

Key `3` is the monotone projection of observed primary-channel means. The consumer's extra exposure `E` is a render parameter. It is not included in this curve.

For keys `c` and `d`, the consumer upsamples both guides with its guided filter (`r = 1`, `eps ≈ 0.05`). It then forms:

```text
q = mix(mix(d, c, l_b), mix(d, c, l_f), m)
```

The maps do not use a fixed affine scale and do not multiply luminance directly. Later shader stages that consume `q` have not been inverted. These means therefore do not solve the local tone transform.

Key `1` uses the basis `[1, R, G, B, R², RG, RB, G², GB, B²]`. Its polynomial reads the displayed primary image and targets thumbnail codes. This is the remove-style direction. Coefficients that local samples cannot identify retain the identity residual. Plane weights use the verified `inputIsLinear = false` formula in `f64`, not half precision. The optional 3×3 spatial filter is not applied.

At background and foreground local tone mapping (LTM) values of 1, the semantic-tone stage returns the primary image. This is the captured `tone=0` setting. The stage does not read the thumbnail or curve, so a tone-zero render cannot measure either resource.

Photos Tone ±1 and Color ±1 slider positions are not source facts. The portable formula check is `apple_style_semantic_tone_channel`. Baseline exposure and base gain retain the source-distribution calibration. The reconstruction does not replace those calibration coefficients.

The owning implementations are [apple_photographic_styles.rs](../crates/xdremux-engine/src/apple_photographic_styles.rs) and [apple_styles.rs](../crates/xdremux-runtime/src/apple_styles.rs).

### Native Styles checks

The native tone experiment runs separately from product conversion. It checks out `21Z121Z1/Photographic_Style_Research` at `37dd551`. It runs `consumer-model/harness/run_tone_sweep.py` on the `xcode-27` image through [research.yml](../.github/workflows/research.yml).

The test sets only `inputToneBias`. It has no color control. The tone values are −1, −0.75, −0.5, −0.35, −0.15, 0, 0.15, 0.35, 0.5, 0.75, and 1.

The Apple samples are `Apple_Native_Samples/IMG_0032.HEIC`, `IMG_0033.HEIC`, and `research/vm-native/samples/IMG_2600.HEIC`, `IMG_2604.HEIC`, and `IMG_2608.HEIC`. Three converted ProXDR fixtures use the same tone values. Before other renders are accepted, tone zero for `IMG_0032.HEIC` must report zero differing half components.

The report records mean luminance, channel means, monotonicity, tone-related plist fields, and `|L(−1)−L(0)| / |L(+1)−L(0)|`.

The product consumer regression uses RGB UHDR and monochrome LHDR fixtures. It decodes the encoded thumbnail and checks the HDR Log target and unchanged primary HEVC. Run it on the required Mac host:

```bash
bash scripts/apple/check_rust_style_consumer.sh
```

When `XDREMUX_STYLES_ACCEPTANCE_DIR` is set, the check retains the actual v15 product HEIC and its native Texture resources. It also retains each fixture's `linear-thumbnail-roundtrip.json` report. These files are diagnostics, not Photos acceptance evidence.

The read-only inspector follows declared file/idat storage and validates extent bounds. It does not patch the product to v16. The separate version-only research control remains a one-byte experiment. Photos import, editing, save, and reopen require device checks.

## Provisional system Gain Map

The default encoder writes XDRemux's own log gain. `XDREMUX_SYSTEM_GAIN_MAP=1` selects a separate prototype.

Rust reconstructs scene-linear Display P3 HDR and builds an HDR appearance with a provisional guided filter. SDR local weight 1 preserves the SDR appearance as an identity transform. Rust then submits both images to the Apple adapter.

The adapter calls `CIContext.writeHEIFRepresentationOfImage` with `.RGBA8` and Display P3 for the SDR image. It submits half-float linear HDR through `kCIImageRepresentationHDRImage`. It sets headroom with `-imageBySettingContentHeadroom:`. The header provides no Swift overlay name for that method.

`kCIImageRepresentationHDRGainMapAsRGB` is true, which requests a color ratio from Core Image. Rust does not supply `kCIImageRepresentationHDRGainMapImage`.

Styles coefficients and the Linear Thumbnail reuse the exact submitted SDR/HDR pair, including SDR quantization and half-float HDR. They do not interpret the new Gain Map with the original OPPO map's channel or scale parameters. Failed readback rejects the measurement before an existing output can be replaced.

### API requirements

The API names and introduction versions come from macOS 27 SDK headers. The inspected headers are `CIContext.h`, `CIImage.h`, `CGImage.h`, `CGImageSource.h`, `CGImageDestination.h`, and `CGImageProperties.h`.

| API or property | SDK introduction version or restriction |
| --- | --- |
| `writeHEIFRepresentationOfImage` | macOS 10.13.4. |
| HDR-image option and `contentHeadroom` | macOS 15. |
| `imageBySettingContentHeadroom` | macOS 26. Headroom in `(0, 1)` is not permitted. |
| `kCGImageDestinationEncodeGenerateGainMapWithBaseImage` | macOS 16. The dictionary value type is undocumented, so the prototype does not call it. |

These versions do not lower the complete product's arm64/macOS 27 requirement or establish availability of private Styles and Texture SPI. Rust sends `max(1, measured peak)` from the half-float values actually submitted to the encoder.

### Readback and unresolved behavior

Readback sets `kCGImageSourceDecodeRequest` to `kCGImageSourceDecodeToHDR`. It sets `kCGImageSourceGenerateImageSpecificLumaScaling` to false, then renders extended-linear Display P3 half-float values.

The report includes both `kCGImageAuxiliaryDataTypeHDRGainMap` and `kCGImageAuxiliaryDataTypeISOGainMap`. Their presence in an output is a measured result, not a consequence of header declarations.

Styles assembly still requires exactly one `tmap` item with its `dimg` graph. If the system output lacks that graph, assembly fails and the report records the conflict. Photos import and editing still require device evidence.

The guided filter is not a port of `guided.py`, which is absent from the public research checkout. Its provisional settings are radius 1, epsilon 0.05, half-size coefficients stored through `f16`, HDR local weight 0, and SDR local weight 1. `XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE` limits prototype geometry. The workflow sets it to 256.

See [system_heif_hdr.rs](../crates/xdremux-runtime/src/system_heif_hdr.rs) and [CoreImageHeifHdr.swift](../Sources/XDRemuxAppleAdapter/CoreImageHeifHdr.swift).

## Apple Portrait

Rust owns source preflight, OPPO block parsing, focus and orientation policy, container processing, Gain Map policy, REND generation, and auxiliary manifests. It also owns routing, output names, validation, and atomic publication. ImageIO reports auxiliary-resource facts. The adapter performs framework operations required by the Rust transaction.

For the complete profile, the adapter submits each Vision segmentation request through a separate image request handler. This avoids the compound-request path where mixed segmentation request types caused an observed macOS 27 beta crash. Required semantic roles and output validation remain unchanged.

A failed request stops the operation. The adapter does not retry that request or substitute the OPPO source-native profile.

### Source-native profile and reduced effects

`--apple-portrait-oppo` uses OPPO source data without Vision or its private segmentation SPI. It still requires macOS because it uses public ImageIO and Core Image operations.

| Resource | Source-native profile |
| --- | --- |
| ISO Gain Map | Preserved from the source. |
| Disparity, focus, aperture, and REND | Derived by Rust from OPPO depth, capture data, and source metadata. |
| Portrait Effects Matte | Added only when the OPPO person plane has credible foreground. |
| Hair matte | Added only when the OPPO hair plane has credible foreground. |
| Skin, teeth, and glasses mattes | Absent. No substitute masks are generated. |

Effects are reduced compared with the complete `--apple-portrait` profile. Low-resolution or missing masks can reduce subject separation and hair detail. Semantic relighting and related edits can be limited.

A depth-only output can satisfy the source-native contract. It is not equivalent to the complete semantic resource set. An OPPO pet plane is never relabeled as a person or hair matte.

The source-native profile never falls back to Vision. Missing required depth or metadata causes failure before publication. The complete profile retains its strict resource requirements. Both profiles use the same Rust conversion and publication path.

The native regression uses two real OPPO Portrait fixtures. ImageIO checks single-file and serial or parallel batch output. The test rejects any attempted Vision operation. It also checks that failed preflight preserves existing output. Run it on the required Mac host:

```bash
python3 scripts/apple/check_rust_cli_oppo_portrait.py
```

## Live Photo and compatibility policy

Ordinary Motion Photo to Live Photo conversion belongs to Rust and does not use the Apple adapter. Any future Apple-only operation on a Live Photo still must preserve Rust ownership of pair identity, validation order, and publication.

Product compatibility decisions remain in Rust. These include source-resource requirements, conflicts with OPPO-compatible output, and publication rules for supported requests. Extending the adapter protocol must not move these decisions into the adapter.

## Validation and acceptance

| Evidence class | What it establishes |
| --- | --- |
| Structural | Expected HEIF/MOV resources and metadata exist and can be parsed. |
| Native framework | The tested Apple framework accepts or exposes the expected resources on that host. |
| Device | The recorded operation works on a specific device and Apple Photos version. |

The completion gate requires the Rust workspace and a real Rust-to-adapter handshake on macOS. Feature checks invoke the Rust CLI, then inspect native consumer results. Structural and native checks do not establish visual equivalence or Photos device acceptance.

CLI acceptance covers commands, media conversion, source-dependent resource contracts, and the Apple adapter. App integration is a separate scope. Reduced source-native effects remain a product tradeoff. Interactive Photos editing requires its own evidence.

Research models such as `ReverseKey1Ensemble` are training assets, not product modes. See the [model card](../research/oppo_styles/models/ReverseKey1Ensemble.model-card.en.md).

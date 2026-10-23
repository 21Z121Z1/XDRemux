# Apple Features

English | [简体中文](apple-features.md)

Photographic Styles and Apple Portrait are Rust-owned product intents in XDRemux. They are not a second product stack.

The canonical public product is the Rust `xdremux` CLI. The Swift target is only the Apple framework adapter for operations that cannot be performed portably, such as ImageIO consumer probing, Vision observations, and VideoToolbox encoding.

## Current availability

The complete XDRemux product, App and Apple adapter require **arm64 / macOS 27 or newer**. The adapter is not built for Intel and checks its supported host before reporting capabilities or executing an operation. The portable Rust core retains Linux, Windows and macOS tests; those checks do not imply Apple feature availability on those hosts. A supported OS and architecture do not replace per-operation API, asset, GPU receipt or final-file validation.

Standard HDR, OPPO-compatible output, Motion Photo → Live Photo, batch processing, categorization, inspection, and portable validation belong to the canonical Rust product.

Photographic Styles and Apple Portrait are expressed as `convert`/`batch` product intents. They do not create Apple-specific CLI subcommands, and low-level adapter or solver controls are not part of the public contract.

Rust owns the feature request/result models, routing, fallback, validation policy, metadata synthesis, assembly, and publication lifecycle. Apple-native code performs only the framework operations requested by Rust and returns factual observations.

## Platform boundary

The CLI uses this structure:

```text
xdremux CLI
    ↓
Rust runtime
    ↓
Rust engine policy
    ↓
portable providers + Apple platform adapter
                         ↓
          ImageIO / Core Image / Vision /
          Core ML / AVFoundation / ...
```

`xdremux-apple-adapter` is a distributable platform component consumed by the Rust product. It is not a user CLI and does not own product policy.

The CLI/runtime uses a versioned, bounded helper-process protocol. Each conversion reuses one lazily started adapter session. Timeout, crash, and framing failures terminate that session. Rust keeps transport private so that a transport change does not change engine or CLI semantics.

## Rust-owned policy

Rust already models the user-level Apple feature intent as two facts: Photographic Styles and Portrait. It does not expose the old Swift producer, donor, backend, or research controls as product configuration.

The first real Apple adapter operation is ImageIO auxiliary-resource probing. The adapter reports observations such as:

- ISO Gain Map presence;
- disparity presence;
- Portrait Effects Matte presence;
- skin, hair, teeth, and glasses semantic matte presence.

The adapter does not answer a business-level question such as “is this a valid portrait output?”. `xdremux-engine` owns that decision through `AppleImageAuxiliaryFacts` and the Portrait resource contract.

New operations must follow this boundary: return the narrowest useful framework fact or operation result, then keep policy in Rust.

## Photographic Styles migration

Rust owns style-generation semantics, constrained search, source-bound policy, key1/property-list synthesis, graph assembly, validation policy, and publication. The adapter is limited to framework observations or encoding primitives requested by the Rust runtime.

Research-only producers, model experiments, donor diagnostics, and RAW experiments remain research tooling. They do not define the public CLI contract or provide a second runtime.

### Texture Styles from the same switch

`--apple-styles` and the app's Photographic Styles switch also generate Texture
Styles. There is no separate Texture option. The app uses `validate --apple-styles` before it skips an existing output. Plain HDR outputs do not satisfy this check. This operation requires the private
Vision and CMImaging APIs tested on macOS 27. A missing API or failed native
operation rejects the whole Styles transaction before publication.

Rust asks ImageIO for an upright sRGB analysis raster with a maximum dimension
of 1024 pixels. This raster is only an analysis input. The output keeps the full
primary image, HDR Gain Map, Styles Linear Thumbnail, and existing item payloads.
The analysis does not reconstruct RAW data or capture-time Apple processing.

Vision supplies face landmarks, signed pose angles, person instances, and semantic
masks. Each measured face is matched to a distinct instance by maximum total
mask overlap. Rust requests the original Mattify, UnderEyeBrighten, and
SkinSmoothStandalone statistics operations for each matched person. Each effect
uses the same inferred geometry and masks. The native order is setup, prewarm,
prepare, calculateStats, process, GPU fence, and terminal finishProcessing.

The runtime adapter uses separate typed requests and results for analysis,
inference, statistics, and final readback. It owns native report decoding and
workfile names. A helper acknowledgement alone cannot supply measured statistics
or admit the final file; the GPU, lifecycle, input bindings, and final-byte
readback checks still apply.

Rust combines only the native statistics that each effect actually returned.
A successful empty optional group is omitted. A scene with no matched faces gets
an empty people array. It does not get a template person. Nose, lips, eyebrows,
and hand protection use regions derived from native landmarks or hand poses.
Ears and tattoo protection are disabled in this derived edit context. This is
not the complete camera FSINC producer.

The Rust HEIF writer adds the version-3 Texture plist, separate encoded masks,
and exact per-instance XMP reference keys to the existing Styles output. Its
tuning profile selects a derived edit; it does not replace EXIF camera identity,
exposure, lens, date, or capture UUID. The writer checks every original item
payload after relocation. Before atomic publication, the original Apple parser
must read back the same person count, reference keys, geometry, and statistics.

Run `python3 scripts/apple/check_texture_styles.py --all-fixtures --output <dir>`
on macOS 27 for real ProXDR fixture conversions, batch conversion, unchanged
primary/Gain Map HEVC, and failure-preserves-output checks. The earlier SDR
research candidates have user-reported Photos activation. That feedback does
not certify every new full-resolution HDR product output, effect response,
save, or reopen on a device.

### SmartStyle edit state

The generated Standard Styles resource set also requires its SmartStyle state
in Apple MakerNote tag 84 (`0x0054`). The coefficient plist is a separate input;
its successful parsing does not establish the photo's edit state. Rust defines
the required integer edit fields, including the Standard Texture state. The ImageIO primitive
merges those requested fields into the existing MakerApple dictionary without
adding a camera identity or UUID. Other MakerApple entries and unmodified cells
remain in the dictionary.

Before publication, Rust stages the final assembled bytes and asks ImageIO to
read their integer dictionary cells. Missing or changed required state rejects
the conversion before replacing an existing output. An older adapter that
ignores the additive metadata request therefore cannot silently publish an
incomplete Styles file. This native readback does not establish Photos UI,
Tone/Color response, or save/reopen behavior; those still require device evidence.

### HDR reference for the Linear Thumbnail

The Styles Linear Thumbnail is an Apple Log encoding of scene-linear Display P3
HDR reconstructed from the SDR base and its Gain Map. Rust applies that gain
map at the base image's original resolution. Gain-map samples are bilinearly
resampled in code space, then the source's ISO 21496-1 / Apple tmap parameters
reconstruct each sample: log2 minimum and maximum, gamma as the reciprocal
exponent used by the 26A434 `_flexMap` decoder, per-channel offsets, and the
alternate-image headroom weight. Reduction to the 1024-pixel-class thumbnail
happens after that reconstruction, as an area box filter in linear light. Rust
then divides each channel by the same `baseline_exposure` stored in Styles
plist key `4` and applies Apple Log. It does not copy SDR image codes into
this resource.

The Apple Log transfer, [0, 1] code range, and existing auxiliary color
signaling stay in place. The encoder is the inverse of the macOS 26A434
exponential decoder (`_appleLogToLinear` / `sw_appleLogToLinear`). That decoder
does not use the quadratic toe, so the encoder does not either. The camera-side
encoder toe is unverified. The transform keeps Display P3 primaries. It does
not change the image to the BT.2020 color space used by Apple Log video or
retag the thumbnail as sRGB, PQ, HLG, or linear.

Coefficients use the decoded display scene. Before encoding the RGB16 Linear
Thumbnail, Rust reverses the primary item's complete rotation/mirror sequence.
The stored Linear Thumbnail and Style Delta Map use the primary coded geometry
and preserve the relative order of its `irot` and `imir` properties. A Gain Map
or Styles item whose aspect does not match the primary coded dimensions is
rejected. `encoding_gain` is renderer metadata, not another exposure division.

The codec primitive receives 16-bit Apple Log codes. VideoToolbox converts
those codes into 10-bit video-range 4:2:0 and encodes Main10 HEVC. The codes
are not quantized to RGB8 before that encode, so the 10-bit range is available
to the encoder. HDR values above SDR white still survive. This remains a
reference derived from the rendered HDR look. It cannot recover RAW or
capture-time processing that the source has lost.

The same 26A434 consumer, `PISemanticStyleFilter`, also decides how the style
metadata is generated:

- Key 3 is one 256-node curve sampled independently on each channel. Its x
  value is the linear thumbnail code `min(S, 1)` after the plist base gain.
  There is no luminance reduction and no percentile normalization. The curve is
  the monotone projection of those observed primary-channel means. The
  consumer's extra exposure `E` is a render parameter and is not folded in.
  Downstream `LinearGTCImage` statistics and the reported fit error use the
  written key-3 nodes after the measured macOS inverse-sRGB decode on the fixed
  65535 scale. They do not use the pre-quantization fit. The 256 decoded nodes
  remain f32, and the repeated final wire word is ignored. The iPhone firmware
  endpoint-normalized half table belongs to a separate binding.
- Keys `c` and `d` stay 32×32. Each cell stores the area mean of one guide:
  `c` is the primary image and `d` is the linear thumbnail. The consumer
  upsamples them with its guided filter (`r = 1`, `eps ≈ 0.05`) and forms
  `q = mix(mix(d, c, l_b), mix(d, c, l_f), m)`. The maps are not scaled by a
  fixed affine and they are not multiplied onto luminance. The later shader
  stages that consume `q` are not inverted, so these means are not a solved
  local tone.
- Key 1 is a 12×9×8×10×3 half array, x-major, with the quadratic basis
  `[1, R, G, B, R², RG, RB, G², GB, B²]`. Each spotlight cell and luminance
  plane is fit separately. The fit is the remove-style direction: the
  polynomial reads the displayed primary and targets the thumbnail code. A
  coefficient the local samples do not identify stays on the identity residual.
  The plane weight uses the verified `inputIsLinear = false` formula in `f64`,
  not half, and the optional 3×3 spatial filter is not applied.

At background and foreground LTM of 1, which is the captured tone=0 setting,
the semantic-tone stage returns the primary and does not read the thumbnail or
the curve. A tone=0 render therefore cannot measure key 3 or the thumbnail.
The Photos Tone ±1 and Color ±1 slider positions are not source facts. The
portable formula check is `apple_style_semantic_tone_channel`.

The native tone check is separate from the product. It checks out
`21Z121Z1/Photographic_Style_Research` at `37dd551` and runs
`consumer-model/harness/run_tone_sweep.py` on the `xcode-27` image. That
harness sets `inputToneBias` only. It does not expose a color control. The
sweep uses tones −1, −0.75, −0.5, −0.35, −0.15, 0, 0.15, 0.35, 0.5, 0.75, and
1. The Apple captures are `Apple_Native_Samples/IMG_0032.HEIC`,
`IMG_0033.HEIC`, and `research/vm-native/samples/IMG_2600.HEIC`,
`IMG_2604.HEIC`, and `IMG_2608.HEIC`. The same tones are applied to the three
ProXDR fixtures after conversion. Tone 0 on `IMG_0032.HEIC` must report zero
differing half components before the other renders are trusted. The report
records mean luminance, per-channel means, `|L(−1)−L(0)| / |L(+1)−L(0)|`,
monotonicity, and the tone-related plist fields. The workflow is
`.github/workflows/research.yml`.

The native consumer regression uses RGB UHDR and monochrome LHDR fixtures,
decodes the encoded thumbnail, and checks the HDR Log target and unchanged
primary HEVC. Run it with `bash scripts/apple/check_rust_style_consumer.sh`.
Photos import, editing, save, and reopen require separate device evidence.
Baseline exposure and base gain still use the existing source-distribution
calibration; this change does not replace those coefficients.

When `XDREMUX_STYLES_ACCEPTANCE_DIR` is set, that check retains the actual v15
product HEIC with native Texture resources and the per-fixture
`linear-thumbnail-roundtrip.json` report. These are diagnostics, not Photos
acceptance. The read-only inspection follows the item's declared file/idat
storage and validates extent bounds; it does not patch the product to v16.
The separate version-only research control remains a one-byte experiment.

### Provisional system gain map

The default encoder still writes XDRemux's own log gain. `XDREMUX_SYSTEM_GAIN_MAP=1`
is a separate prototype. Rust reconstructs scene-linear Display P3 HDR, builds
an HDR look with a provisional guided filter, and keeps the SDR look by using
local weight 1, which is the identity. It then hands both images to the Apple
adapter. The adapter calls `CIContext.writeHEIFRepresentationOfImage` (`format`
`.RGBA8`, Display P3) and passes the half-float linear HDR image under
`kCIImageRepresentationHDRImage`. Headroom is applied with the header method
`-imageBySettingContentHeadroom:` (macOS 26). The header does not publish a
Swift overlay name for that method.
`kCIImageRepresentationHDRGainMapAsRGB` is set true so Core Image is asked for
a color ratio. Rust does not supply `kCIImageRepresentationHDRGainMapImage`.

Styles coefficients and the Linear Thumbnail reuse the exact SDR/HDR pair
submitted to the system encoder, including SDR quantization and the submitted
half-float HDR. They do not reinterpret the newly generated Gain Map with the
original OPPO map's channels or scale parameters. Failed readback rejects a
measurement before an existing output can be atomically replaced.

Those names and their availability come from the macOS 27 SDK headers
(`CIContext.h`, `CIImage.h`, `CGImage.h`, `CGImageSource.h`,
`CGImageDestination.h`, `CGImageProperties.h`), not from a guessed selector.
`writeHEIFRepresentationOfImage` is macOS 10.13.4. The HDR-image option and
`contentHeadroom` are macOS 15, and `imageBySettingContentHeadroom` is macOS 26.
These are SDK introduction versions for individual APIs, not the support floor
for the complete arm64/macOS 27 product or its private Styles/Texture SPI.
Headroom sent by Rust is `max(1, measured peak)` of the half values handed to
the encoder. The header forbids a headroom in `(0, 1)`. ImageIO
`kCGImageDestinationEncodeGenerateGainMapWithBaseImage` exists from macOS 16,
but its dictionary value type is not documented, so the prototype does not
call it.

Readback uses `kCGImageSourceDecodeRequest` = `kCGImageSourceDecodeToHDR` and
sets `kCGImageSourceGenerateImageSpecificLumaScaling` to false, then renders
extended-linear Display P3 half. It reports both
`kCGImageAuxiliaryDataTypeHDRGainMap` and `kCGImageAuxiliaryDataTypeISOGainMap`.
Which of those the system file actually contains is a measurement, not a
header fact. The Styles assembler still requires one `tmap` item and its
`dimg` graph. If the system file does not have that graph, assembly fails
closed and the report records the conflict. Photos import and editing remain
device evidence.

The guided filter is not a port of `guided.py`; that file is not in the public
research checkout. Radius 1, epsilon 0.05, half-size coefficients stored
through `f16`, HDR local weight 0, and SDR local weight 1 are provisional.
`XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE` bounds the prototype geometry. The workflow
sets it to 256.

## Apple Portrait migration

Rust owns Portrait preflight, OPPO block parsing, focus/orientation policy, JPEG/container logic, Gain Map policy, REND generation, auxiliary-manifest construction, feature routing, output naming, validation policy, and atomic publication. ImageIO reports auxiliary-resource facts and the adapter performs only the Apple framework operations required by the Rust transaction.

For the complete Portrait profile, the adapter submits each Vision segmentation request through a separate image request handler. This avoids batching different segmentation request types through the compound request path where a macOS 27 beta crash was observed. The requested semantic roles and output validation remain the same. A failed request stops the operation; the adapter does not retry it or substitute the source-native profile.

### Source-native profile and reduced effects

`convert` and `batch` accept `--apple-portrait-oppo`. This profile does not call Vision, including its private segmentation SPI. It uses public ImageIO and Core Image operations, so macOS is still required.

| Resource | Source-native profile |
| --- | --- |
| ISO Gain Map | Preserved from the source. |
| Disparity, focus, aperture and REND | Derived by Rust from OPPO depth, capture data and source metadata. |
| Portrait Effects Matte | Added only when the OPPO portrait plane has credible foreground. |
| Hair matte | Added only when the OPPO hair plane has credible foreground. |
| Skin, teeth and glasses mattes | Absent. No substitute masks are generated. |

Portrait effects are reduced compared with the complete `--apple-portrait` profile. Low-resolution or missing producer masks can reduce subject separation and hair detail. Semantic relighting and related edits can be limited. A depth-only output is valid for this source-native resource contract; it is not equivalent to the complete semantic resource set. An OPPO pet plane is not relabeled as a person or hair matte.

The native profile never falls back to Vision. Missing required depth or source metadata causes an error before output publication. The complete profile keeps its existing strict resource requirements. Both profiles use the same Rust conversion and publication path.

The native regression uses two real OPPO Portrait fixtures, checks single-file and serial/parallel batch output with ImageIO, and rejects any attempted Vision operation. It also verifies that a failed preflight preserves an existing output. Run it with `python3 scripts/apple/check_rust_cli_oppo_portrait.py`.

## Live Photo

Normal Motion Photo → Live Photo conversion is already a Rust product capability and should not be routed through the Apple capability adapter.

If a future combined feature requires applying an Apple-only operation to a Live Photo still, Rust must continue to own the Live Photo asset lifecycle, pair identity, publication, and validation ordering. The Apple adapter should receive only the narrow platform operation it needs to perform.

## Compatibility rules

Product-level compatibility rules belong in Rust. Examples include whether OPPO-compatible output can be combined with an Apple editing feature, whether a source asset contains the resources required for Portrait editing, and how a combined request is published atomically.

Do not move these decisions into the adapter protocol when extending the CLI.

## Validation and acceptance

Use three distinct evidence classes:

1. Structural evidence proves that HEIF/MOV resources and metadata are present and parseable.
2. Native framework evidence proves that the tested Apple framework accepts or exposes the expected resources.
3. Device evidence proves behavior in a specific Apple Photos version on a real device.

Structural evidence does not replace device evidence for an interactive Apple Photos editing claim.

The canonical completion gate requires the Rust workspace and a real Rust → Apple adapter handshake on macOS. Feature-specific gates drive the Rust CLI and then query Apple consumer facts. Structural and native-framework evidence do not by themselves claim visual equivalence or Photos device acceptance.

CLI migration acceptance covers the canonical Rust commands, media conversion, source-dependent resource contracts and the Apple adapter. macOS app integration is a separate scope. Native-profile quality reduction is a documented product tradeoff. Interactive Photos editing remains a separate consumer claim; a CLI pass does not assert identical Photos effects or editing UI.

## Research material

The repository still contains style research code and optional models such as `ReverseKey1Ensemble`. These are research/training assets rather than product modes. See the [model card](../research/oppo_styles/models/ReverseKey1Ensemble.model-card.en.md) where relevant.

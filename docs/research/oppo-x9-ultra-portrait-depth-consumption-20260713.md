# OPPO Find X9 Ultra Portrait Depth Consumption

Date: 2026-07-13

## Scope and result

This note traces both sides of the Find X9 Ultra rear-portrait contract: how
the camera algorithm produces and serializes the private payload, and how
Gallery consumes it for refocus/rendering. It separates confirmed code paths
from fields whose exact native semantics remain unresolved.

The main result is that OPPO does not treat `rear.depth` as one standalone
grayscale depth map and does not render blur from a single physical camera
calibration formula. The X9 Ultra path is:

```text
HEIC FileExtendedContainer
  -> Gallery PortraitBlurHelper
  -> BackBlurProcessUnit / BackBlurProcessUnitSDK
  -> APSClient transact (back_blur)
  -> libAlgoProcess.so
  -> libOPAlgoCamCaptureDualPortrait.so
  -> Zstd package decode
  -> rank + hair/portrait/pet/other auxiliary data
  -> focus-depth selection
  -> zoom-dependent nonlinear depth-to-CoC mapping
  -> semantic/hair refinement and PSF rendering
```

This is the missing distinction behind the Apple conversion experiments:
OPPO's per-capture disparity geometry, OPPO's semantic package, and OPPO's
render curve are three separate layers.

## Evidence set

The static trace uses the local X9 Ultra firmware/Gallery artifacts:

- Gallery Java decompile under
  `tmp/oppo_gallery_full_decode_reverse_20260705/jadx/sources`;
- `libAlgoProcess.so` extracted from the X9 Ultra firmware;
- `/odm/lib64/libOPAlgoCamCaptureDualPortrait.so`;
- `/odm/etc/camera/dualcam_capture_bokeh/render_param.json`;
- the deduplicated 80-original OPPO portrait corpus, including
  `IMG20260713001840` and `IMG20260606175915`.

Relevant SHA-256 values for the inspected firmware build are:

- `libAlgoProcess.so`:
  `9c82a2808b0fa62131b4ee8a424041368904ddb3870dd55311d737c9b5e8da2d`;
- `libOPAlgoCamCaptureDualPortrait.so`:
  `f0e58e9df925f360022486788c5b919be898c4ffe5cd46faea510745e363c939`;
- `render_param.json`:
  `8281b0db899e8f952438c3a4768470c88b5a4f7039d65fd6bb3ba3c478ecca23`.

Large string and disassembly dumps stay under
`/private/tmp/oppo_depth_reverse_20260713` and
`/private/tmp/oppo_x9_portrait_producer_20260713`. They are intentionally not
added to the repository.

## Capture-side producer and package writer

The Camera APK is not the depth generator. Its Java layer reads the completed
`com.oplus.rear.depth` and `com.oplus.rear.depth.config` byte arrays from the
capture result and copies them into `FileExtendedContainer`. Generation and
serialization happen in the ODM/APS native stack.

The producer-side symbols and call sites are now identified in
`libOPAlgoCamCaptureDualPortrait.so`:

- `galleryModeSetDepthInfo` at `0x7d5390` fills the `DepthInfo`/`RenderInfo`
  header from the capture algorithm state;
- `bokehGetGellerySaveDataLen` at `0x7e01c0` calculates the complete gallery
  package length;
- `bokehGetGellerySaveDataBuf` at `0x7e0970` serializes the package;
- the final buffer is compressed with `ZSTD_compress(..., level=1)` before the
  Camera APK receives it.

The uncompressed package is written in this order:

| Order | Payload | Role in the camera pipeline |
|---:|---|---|
| 1 | `0x300` header | `DepthInfo` (`0x118` bytes) plus `RenderInfo` at `0x180` (`0xd0` bytes) |
| 2 | rank plane | compact relative disparity/depth rank used for Gallery refocus |
| 3 | hair plane, when flagged | hair-aware foreground refinement |
| 4 | portrait plane, when flagged | selected human-subject topology |
| 5 | pet plane, when flagged | selected animal-subject topology |
| 6 | semantic segmentation, when present | producer converts its wider labels to a stored uint8 semantic plane |
| 7 | motion-mask block, when present | motion/occlusion reliability state and its table data |
| 8 | spotlight block, when present | `0x38` bytes of spotlight info followed by its image |
| 9 | rectified master and slave NV21 | stored-resolution stereo guide frames, each `w*h*3/2`; native log names them `master` and `slave` |
| 10 | optional model-output image | a later `w*h*3/2` buffer when its header flag is set; static evidence does not identify it as a rendered-bokeh image |

This closes the largest earlier uncertainty: the remaining 4-7 MiB is mostly
rectified master/slave YUV plus semantic/motion data and, in some captures, an
optional model-output image. It is not one hidden high-resolution metric-depth
map. In `IMG20260506112827`, the decoded package ends exactly after the two
1024x768 rectified NV21 frames, so that sample contains no third image. The
outer HEIC primary is the confirmed final OPPO-rendered portrait.

## Gallery entry and backend selection

Gallery reads the following portrait resources from `FileExtendedContainer`:

- `src.image`;
- `rear.depth`;
- `rear.depth.config`;
- `rear.spotlight` when present;
- HDR transform and linear-mask resources when present.

`PortraitBlurHelper` creates an origin image description from `src.image`, a
gray depth description using the dimensions from `rear.depth.config`, and a
gray spotlight description. Rotation comes from `cameraRoll`; mirroring comes
from `mirrorEnable`.

The X9 Ultra-supported path is `RearBlurNode`, backed by the camera IPU/APS
stack. `ArcSoftRearBlurNode` is a compatibility fallback for devices on which
`IPUFeature.BACK_BLUR` is unavailable. Conclusions from the ArcSoft JNI alone
must therefore not be used as the X9 Ultra product behavior.

## `rear.depth.config` contract

`RearDepthStruct` is parsed sequentially in Gallery. The stable fields are:

| Version | Fields introduced |
|---|---|
| base | version; depth width/height; focus x/y; 32 `blurApertures`; 32 `blurValue`; `blurStrength`; `cameraRoll` |
| 2.0 | spotlight width/height; `fNumber`; `distance`; tele-master flag; reference/min EV; scene mode |
| 2.2 | four-value focus rectangle and valid flag |
| 2.3 | mirror flag |
| 2.4 | refocus mode; light-spot controls; `curveVal`; shine and spot-sharpen controls; `foregroundBlurScale`; master type |
| 2.5 | big-face, pet, and multi-semantic-segmentation enables |
| 4.0 | bokeh version; ISO; zoom ratio; focus ROI type; shutter; lux; face rectangles/angles; 296 keypoints per face and confidence/inter values |

For the v4 sample `IMG20260713001840`:

- `fNumber` is at byte offset 292 and is `6.3`;
- `distance` is at byte offset 296 and is `102`;
- the selected `blurStrength` is `30`;
- the aperture/strength table contains 22 active pairs:

```text
f/16 -> 1,  f/14 -> 3,  f/13 -> 5,  f/11 -> 9,
f/10 -> 12, f/9 -> 17,  f/8 -> 22,  f/7.1 -> 26,
f/6.3 -> 30, f/5.6 -> 35, f/5 -> 44, f/4.5 -> 52,
f/4 -> 60, f/3.5 -> 70, f/3.2 -> 81, f/2.8 -> 96,
f/2.5 -> 99, f/2.2 -> 111, f/2 -> 120, f/1.8 -> 130,
f/1.6 -> 140, f/1.4 -> 150
```

Gallery does not regenerate this curve. The aperture slider selects one array
index and sends both values:

- `f_aperture = blurApertures[index]`;
- `blur_strength = blurValue[index]`.

The original/default state similarly uses `fNumber` and `blurStrength` from
the config. Both values are serialized into the APS `back_blur_process`
parameter array. They are not interchangeable: `f_aperture` is visibly used
by the native depth-to-radius path, while `blur_strength` remains a separate
process control.

The bundled ArcSoft compatibility backend independently confirms that the
second value is not UI-only metadata. In
`libdualcam_refocus_gallery_jni.so`, `common_Native_Process` reads both
`f_aperture` and `blur_strength` from the Java parameter object, clamps
`blur_strength` to `0...200`, and passes it together with the aperture,
refocus point, and focus rectangle to `ARC_DCIR_Process`. This does not prove
that the X9 Ultra OPLUS backend uses the same ArcSoft formula, but it does
prove the Gallery contract is deliberately two-dimensional rather than an
aperture label plus a redundant lookup value.

In the current OPLUS library, `bokehCapWrapper::setBokehParameters` dispatches
effect-parameter type 2 to `bokehSetBokehEffectPara`, while the recovered
depth-to-radius state and logs expose the name `Fnumber`. Static analysis has
not yet tied the incoming `blur_strength` string-array entry to a separately
named member in that structure. It may remain a second coefficient, or APS may
first translate the paired values into the effect structure. The absence of a
`blurStrength` symbol in the final library is not evidence that Gallery's
value is ignored.

## APS bridge

`BackBlurProcessUnitSDK` declares algorithm `OPLUS_BOKEH`, mode `back_blur`.
Its init request carries the complete depth byte array, origin/spotlight data,
and the v4 camera/face/keypoint parameters. Its process request carries:

- refocus x/y and focus rectangle;
- face rectangles;
- output dimensions;
- `blur_strength`;
- `f_aperture`;
- optional HDR/linear-mask controls.

The SDK uses APS transactions 4098 through 4102 for create, init, process,
uninit, and destroy. `libAPSClient-cmd-jni.so` loads `libAlgoProcess.so`.
Static references in `libAlgoProcess.so` confirm that the callback/extension
path transports `rear.depth`, `rear.depth.config`, their addresses, and their
sizes into the bokeh algorithm graph.

## Where Zstd is actually decoded

The Zstd decode is not performed by `RearDepthStruct`, and the generic
`decompress` log in `libAlgoProcess.so` belongs to JPEG/EXIF debug-data code.
The actual portrait-depth decoder is confirmed in:

```text
libOPAlgoCamCaptureDualPortrait.so
  bokehSetBokehEffectPara @ 0x7c3690
```

The function:

1. reads the compressed depth pointer and input length;
2. calls `ZSTD_getFrameContentSize`;
3. allocates exactly that output size;
4. calls `ZSTD_decompress` and `ZSTD_isError`;
5. parses a fixed 768-byte package header;
6. assigns the first data plane at `decoded + 0x300`.

This corrects the earlier shorthand that Gallery hands native code an already
decoded gray image. Gallery labels the resource as gray for the process-unit
contract, but the algorithm receives and decodes the complete Zstd package.

## Decoded package layout

The portion confirmed from both real data and native pointer arithmetic is:

| Offset | Meaning | Confidence |
|---:|---|---|
| `0x000` | primary rank width | confirmed |
| `0x004` | primary rank height | confirmed |
| `0x018` | per-rank disparity scale | confirmed by corpus behavior |
| `0x01c` | effective depth focal length | confirmed by continuous focal correlation |
| `0x020` | stereo/depth baseline profile | confirmed by lens-pair clustering |
| `0x024` byte | hair-mask-present flag | confirmed by native log and `SceneDetail.setHairExist` |
| `0x025` byte | portrait-mask-present flag | high confidence from pointer slot and native `portraitmask` log |
| `0x026` byte | pet-mask-present flag | confirmed by `SceneDetail.setPetExist` |
| `0x027` byte | near-object flag | confirmed by producer log and focus-depth branch |
| `0x028` float | near-object confidence | confirmed by producer log |
| `0x02c` byte | plant-object flag | confirmed by producer log |
| `0x02e/0x030` uint16 | `disp2depthMin` / `disp2depthMax` quantization endpoints | confirmed by producer kernel/log |
| `0x032` byte | rank exponentiation mode | confirmed by producer conversion path |
| `0x188/0x18c` | dimensions for a later optional auxiliary image | confirmed |
| `0x190` byte | presence flag for that auxiliary image | confirmed |
| `0x1b0` | scene class | confirmed by `SceneDetail.setSceneClass` call |
| `0x1b4` | capture-side `object_distance` | confirmed by producer store/log; zero in all 80 saved samples |
| `0x1b8` | AEC lux index | confirmed by producer store/log |
| `0x1bc` | app zoom ratio / zoom bucket | confirmed by native logging and real focal groups |
| `0x300` | first uint8 rank plane | confirmed |

After the main `width * height` rank plane, the parser advances by another
`width * height` for each enabled hair, portrait, and pet plane. Producer-side
serialization proves that the later payload is then semantic segmentation,
motion-mask state, optional spotlight data, rectified master/slave NV21 and an
optional model-output image. Other native probability, monocular-depth, confidence,
stripe and hollow-mask objects are generation/refinement intermediates; static
evidence does not prove that each is serialized as a separate gallery plane.

The expanded 80-original corpus confirms the size range. After consuming rank
and every flagged same-size hair/portrait/pet plane, each package still contains
approximately `4.10...7.31 MiB` (median `6.19 MiB`), now explained primarily by
the rectified NV21 pair plus variable semantic/motion data and optional image
blocks. All 80 set the portrait-plane flag, but
only 51 have nonzero portrait content; two have a nonzero pet plane and 44 have
a nonzero hair plane. Current XDRemux therefore uses OPPO subject topology for
53/80 originals and Vision person segmentation for the remaining 27.

The producer fields also correct two earlier assumptions:

- all 80 saved packages have `exponentiation=1`, so their rank-to-relative-
  disparity conversion is linear; a future non-1 package must use the inverse
  nonlinear conversion rather than the current linear formula;
- six of the 80 set the near-object flag (saved confidence `0.5`); none sets
  the plant-object flag, so the near-object branch is real but sparse in this
  corpus;
- the header's `object_distance` at `0x1b4` is zero in all 80 files, while
  `rear.depth.config.distance` is populated. The latter is therefore the only
  useful saved distance prior in this corpus. `0x1b8` is AEC lux, not distance.

The embedded producer OpenCL kernel `cvt_flt_disp_to_u8_dpt` also gives the
exact saved-rank normalization:

```text
normalized = (65535 - internalDisp16 - disp2depthMin)
             / (disp2depthMax - disp2depthMin + 1e-5)
rank = 255 * normalized                 # exponentiation = 1
rank = 255 * sqrt(normalized)           # exponentiation = 2
```

Thus the quantized internal focus disparity can be reconstructed as:

```text
normalizedFocus = (focusRank / 255) ^ exponentiation
internalFocusDisp16 = 65535 - (disp2depthMin
                      + normalizedFocus * (disp2depthMax - disp2depthMin))
```

This is the missing absolute quantization anchor; `depthScale` still describes
the exported relative-disparity step. The internal value must remain a fitting
feature until its upstream `quantScale`, offset and rectify-domain conversion
are fully recovered—it is not yet safe to write directly as Apple disparity.

Consequences:

- preserving the whole `rear.depth` is required for OPPO-native re-editing;
- the current Apple conversion consumes rank plus the confirmed same-size
  portrait/pet/hair planes, while intentionally omitting camera-algorithm guide
  YUV and any optional model-output image;
- Vision-generated PEM remains a necessary fallback for empty subject planes,
  not an exact substitute for OPPO's complete packed semantic state.

## How OPPO turns depth into blur

The native renderer first establishes `focusDepth`. It has explicit routines
for center point, face, portrait, portrait-without-face, pet face, histogram,
and near-object scenes. Native logs also state that face landmarks can update
`focusDisp`. The config fields therefore divide into distinct roles:

| OPPO field | Native role | Apple-conversion use |
|---|---|---|
| `focusX/focusY` and valid focus rect | initial focus ROI | map to the rank plane and Apple Focus XMP |
| face rectangles, angles and 296 keypoints | select/verify the focused face and robust focus rank | prefer face/eye-region rank when it intersects the focus ROI |
| portrait/pet masks | choose the focused subject component | gate the focus-rank statistic and build PEM |
| near-object flag/confidence | switch to the near-object focus-depth branch | select a near-object-specific robust-rank fallback |
| `distance` | scene/object-distance prior | sanity-check or fit the per-profile focus scalar; do not copy it directly into Apple REND |
| 22-point aperture/blur-strength table | coupled OPPO UI aperture and renderer-strength curve | initial aperture plus a future Apple aperture-response fit |
| `foregroundBlurScale` | feeds foreground CoC controls | tune foreground disparity/CoC response; it is not a focus-distance field |
| face/portrait/pet semantic data | focus selection and boundary refinement | Focus, PEM and semantic hair, with Vision only as fallback |

The 80-file corpus further confirms that `focusX/focusY` are expressed in
`src.image` raw JPEG coordinates, even though the config also declares a
`900x1200` processing canvas. The focus coordinates reach `3040x2601`; mapping
them through `900x1200` incorrectly clamps the focus sample to the depth edge.
The rank plane shares the `src.image` storage direction, so the correct mapping
is direct normalized raw-image coordinates, with EXIF orientation applied only
for display/UI coordinates.

Using that corrected mapping, the relative focus disparity
`(255 - medianFocusRank) * depthScale` correlates with inverse
`rear.depth.config.distance` across the corpus (`Pearson r=0.57` overall).
Within one physical module the relation is clearer: approximately `0.69` for
7.73 mm, `0.68` for 20.1 mm and `0.87` for 34.8 mm. This is strong enough to
use distance as a profile-specific consistency prior, but not strong enough to
replace the actual local rank/semantic focus selection.

The reconstructed `internalFocusDisp16 / headerFx` is even more strongly
correlated with inverse config distance for the 7.73 mm and 20.1 mm modules
(`r=0.985` and `0.979`). The 34.8 mm/230 mm module does not follow that same
single relation; its distance modes continue to depend on header scale and the
6-10x native branch. This is direct evidence for a per-profile/scene fit rather
than one global distance multiplier.

The embedded OpenCL source in the X9 Ultra bokeh library shows the core
background relation:

```text
inputData = realDepth - focusDepthOld
baseBackground = bgCoef * inputData / (inputData + focusDepth)
k = (1 + inputData/focusDepth)
    / (1 + inputData/(focusDepth * tblVal))
blur = clamp(k * shapedBackground, fb_MAX_BLUR, 60)
```

Foreground blur uses a separate `fgCoef`, `Hasselblad_Len`, foreground enable,
and smoothing branch. The background shaping differs by app zoom:

- 1x to below 2x;
- 2x to below 3x;
- 3x to below 6x;
- 6x through 10x.

The branches use different combinations of `distSmooth`, `distSmooth2`,
`dist1Xsmooth`, slope/clamp behavior, and near-object blending. This proves
that digital focal lengths are not handled by choosing the nearest optical
camera point and applying one linear disparity multiplier.

## Firmware render calibration

The firmware file used by this renderer is:

```text
/odm/etc/camera/dualcam_capture_bokeh/render_param.json
```

For Lighthouse/X9 Ultra it contains:

```json
{
  "Hasselblad_Len_Table_phone": [30, 45, 66, 80, 116, 139, 220],
  "zoomMode_to_Len_lut": [3, 0, 2, 5],
  "Phone_Len": [1.0, 1.5, 2.0, 3.0, 3.6, 6.0, 10.0]
}
```

The file itself says only `Hasselblad_Len_Table_phone` is intended to be
modified and that it must remain monotonic. Native code calls
`renderParseJsonFile` and `updateHasselbladLen` before converting depth to
radius. This table is a render-domain lens model; it is not equivalent to the
Apple auxiliary camera-calibration dictionary and should not be copied into
that dictionary field-for-field.

## The important f/16 special case

`GetBlurmapEngine::convertDepthToRadius` contains an explicit endpoint rule.
When the incoming aperture evaluates to f/16, native replaces the internal
calculation value with:

- f/40 below approximately 6x;
- f/20 from approximately 6x upward.

The UI may still display f/16. The replacement happens inside the blur-map
calculation and is logged as `mBlurmapParams.Fnumber` versus `used Fnumber`.

This explains the device comparison:

- OPPO can make its f/16 endpoint nearly unblurred even with long-focal depth;
- the Apple conversion only sets Apple's displayed aperture and leaves the
  donor renderer's response curve unchanged;
- changing Apple camera calibration alone cannot reproduce OPPO's f/16
  endpoint and can amplify long-focal blur dramatically.

## Implications for XDRemux / Apple portrait conversion

1. Keep `src.image`, the complete compressed `rear.depth`, and the complete
   `rear.depth.config` untouched when the user has not enabled Apple portrait
   conversion.
2. For Apple conversion, continue using the primary rank plane plus header
   scale as source geometry; do not insert OPPO physical focal/baseline values
   into a mismatched Apple private render graph.
3. Use the `blurApertures[] / blurValue[]` table as the source aperture curve,
   but do not assume one constant `renderGain` is sufficient.
4. Model at least zoom region, focus depth/distance class, foreground versus
   background response, and the f/16 endpoint behavior.
5. Map confirmed portrait/pet/hair planes first and treat Vision PEM generation
   as a fallback compatibility layer. Do not claim that either currently
   preserves all later OPPO semantic refinement data.

## Mapping OPPO focus state into Apple REND

The controlled Apple aperture series rules out edited aperture as the source
of REND records `0x01c2...0x01c5`. A second controlled set (`IMG_7303...7310`)
keeps scene and capture aperture fixed while tapping near, middle and far
subjects at 1x, 2x and 3x. All eight originals have distinct hashes. Both the
`0x0190...0x0193` and `0x01c2...0x01c5` families change between refocus
captures, proving that they encode per-capture focus/scene state rather than
edited aperture alone.

Several exact profile relationships remain stable:

| Apple profile | `0x01c3 / 0x01c2` | `0x01c4 / 0x01c2` | `0x01c5 / 0x01c2` |
|---|---:|---:|---:|
| 1x / 24 mm | `2.5` | `0.075` | not constant; a second profile/scene term remains |
| 2x / 48 mm | `2.875` | `0.0875` | not constant; `0.835...1.051` in the refocus set |
| 3x / 77 mm | `2.875` | `0.0875` | `0.5` |
| 5x / 120 mm sample | `2.875` | `0.0875` | `1.5667273` in the available capture |

The other dynamic family has equally strong structure:

- `0x0192 = 48 * 0x0191` for all eight captures;
- `0x0193 = 1.0 * 0x0191` at 1x;
- `0x0193 = 0.4 * 0x0191` at 2x and 3x;
- `0x0190` and `0x0191` both move with refocus at 1x/2x, while the two 3x
  captures keep them at `50` and `0.25` even though `0x01c2` changes.

Therefore the graph contains at least two independent per-scene controls. The
four `0x01c2...0x01c5` floats must not be independently guessed, but fitting
only one scalar is also insufficient at 1x/2x. OPPO's integer `distance` must
not be copied directly into any one record. The safe implementation shape is:

```text
focusRank = nativeLikeFocusSelector(rank, focusROI, faceKeypoints,
                                    portrait, pet, nearObject)
focusDisparity = inverseRank(focusRank, depthScale, exponentiation)
sceneScalar = profileFit(focusDisparity, 1/configDistance,
                         focusBranch, nearObjectConfidence)
blurScene = profileBlurFit(focusDisparity, 1/configDistance,
                           focusBranch, foregroundBlurScale)

rend[0x01c2] = sceneScalar
rend[0x01c3] = profileC3 * sceneScalar
rend[0x01c4] = profileC4 * sceneScalar
rend[0x01c5] = profileC5(sceneScalar, secondarySceneState)
rend[0x0190] = quantizedProfileBlurState(blurScene)
rend[0x0191] = normalizedProfileBlurState(blurScene)
rend[0x0192] = 48 * rend[0x0191]
rend[0x0193] = profile0193Ratio * rend[0x0191]
```

Raw Apple focus-pixel disparity is not a cross-capture absolute anchor. In the
1x set it remains around `0.82...0.89` while `0x01c2` spans `2.26...3.61`; in
the 2x set neither quantity is monotonic with the other. Each frame's relative
disparity gauge can shift, and the XMP Focus point is not necessarily the same
semantic focus component used by the renderer. The current representative REND
values should therefore remain until the two fits are trained from controlled
distance/refocus captures using normalized rank, quantization endpoints,
semantic-selected focus component and a metric-distance prior. OPPO producer
and config data provide these inputs; the remaining inverse problem is fitting
them into Apple's two private scene controls.

## Confirmed versus remaining unknown

Confirmed:

- the capture-side native package writer and Zstd compression call;
- the X9 Ultra IPU/APS call chain;
- complete v4 config transfer;
- native Zstd decode location;
- the 768-byte header and primary plane boundary;
- hair/portrait/pet optional plane routing;
- semantic/motion/spotlight and rectified-master/slave/bokeh NV21 serialization
  order;
- the meanings of saved `object_distance`, AEC lux, app zoom, near-object,
  min/max quantization endpoints and exponentiation fields;
- zoom-dependent nonlinear depth-to-radius logic;
- firmware render calibration table;
- special internal handling of f/16;
- independent JNI evidence that `blur_strength` is an active bounded process
  parameter, not display-only metadata.

Still requiring a device hook or further type recovery:

- the exact native function that serializes the separate
  `rear.depth.config` blob (its complete field contract and Java byte-array
  transport are confirmed, but the producer symbol is not yet named);
- authoritative C names for the still-unidentified fields in the 768-byte
  package header and exact sub-layout of the variable motion/spotlight blocks;
- the exact mathematical contribution of `blur_strength` inside the current
  X9 Ultra OPLUS backend after APS parsing (the fallback ArcSoft backend is
  confirmed to consume it, but its formula is not transferable);
- every coefficient selected by zoom/scene from the native calibration state;
- a direct numerical mapping from OPPO focus state and aperture curve to
  Apple's private per-scene REND scalar(s).

The next high-value OPPO hook is at `bokehSetBokehEffectPara` after
`ZSTD_decompress`, plus `GetBlurmapEngine::convertDepthToRadius`. Logging the
selected focus branch, `focusDepth`, `bgCoef`, `fgCoef`, `tblVal`, incoming and
used f-number would close the remaining OPPO renderer model. Separately, the
Apple REND inverse needs controlled fixed-profile captures at several physical
distances/refocus points; OPPO static analysis cannot reveal an Apple-private
transfer function that is not present in this firmware.

# OPPO Find X9 Ultra Portrait Depth Consumption

Date: 2026-07-13

## Scope and result

This note traces how the Find X9 Ultra Gallery build consumes rear portrait
data from the HEIC private tail. It separates confirmed code paths from fields
whose exact native semantics remain unresolved.

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
- three real decoded `rear.depth` packages, including
  `IMG20260713001840` and `IMG20260606175915`.

Relevant SHA-256 values for the inspected firmware build are:

- `libAlgoProcess.so`:
  `9c82a2808b0fa62131b4ee8a424041368904ddb3870dd55311d737c9b5e8da2d`;
- `libOPAlgoCamCaptureDualPortrait.so`:
  `f0e58e9df925f360022486788c5b919be898c4ffe5cd46faea510745e363c939`;
- `render_param.json`:
  `8281b0db899e8f952438c3a4768470c88b5a4f7039d65fd6bb3ba3c478ecca23`.

Large string and disassembly dumps stay under
`/private/tmp/oppo_depth_reverse_20260713`. They are intentionally not added
to the repository.

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
| `0x188/0x18c` | dimensions for a later optional auxiliary image | confirmed |
| `0x190` byte | presence flag for that auxiliary image | confirmed |
| `0x1b0` | scene class | confirmed by `SceneDetail.setSceneClass` call |
| `0x1b8` | focus-distance-like float used by render setup | high confidence |
| `0x1bc` | app zoom ratio / zoom bucket | confirmed by native logging and real focal groups |
| `0x300` | first uint8 rank plane | confirmed |

After the main `width * height` rank plane, the parser advances by another
`width * height` for each enabled hair, portrait, and pet plane. It continues
through additional optional buffers described by later header fields. One
later path computes `width * height * 3 / 2`, proving that an optional NV21/YUV
image can also be embedded in the decoded package.

The three inspected packages decode to roughly 9.2-10.0 MB although their
primary rank plane is only 786,432 bytes. This is direct evidence that the
package contains much more than the rank plane. The exact names of every later
buffer are not yet safe to claim, but native logs and symbols show consumers
for portrait probability, semantic label, monocular depth, confidence,
periodic/stripe mask, hollow mask, hair mask, pet mask, and guide YUV data.

The expanded 80-original corpus confirms that the unresolved remainder is not
an isolated sample artifact. After consuming rank and every flagged same-size
hair/portrait/pet plane, each package still contains approximately
`4.10...7.31 MiB` (median `6.19 MiB`). All 80 set the portrait-plane flag, but
only 51 have nonzero portrait content; two have a nonzero pet plane and 44 have
a nonzero hair plane. Current XDRemux therefore uses OPPO subject topology for
53/80 originals and Vision person segmentation for the remaining 27.

Consequences:

- preserving the whole `rear.depth` is required for OPPO-native re-editing;
- the current Apple conversion consumes rank plus the confirmed same-size
  portrait/pet/hair planes, but still leaves the later package buffers unused;
- Vision-generated PEM remains a necessary fallback for empty subject planes,
  not an exact substitute for OPPO's complete packed semantic state.

## How OPPO turns depth into blur

The native renderer first establishes `focusDepth`. It has explicit routines
for center point, face, portrait, portrait-without-face, pet face, histogram,
and near-object scenes. Therefore `rear.depth.config.distance` is an input to
scene/render setup, not a Java-side formula that directly scales every rank.

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

## Confirmed versus remaining unknown

Confirmed:

- the X9 Ultra IPU/APS call chain;
- complete v4 config transfer;
- native Zstd decode location;
- the 768-byte header and primary plane boundary;
- hair/portrait/pet optional plane routing;
- zoom-dependent nonlinear depth-to-radius logic;
- firmware render calibration table;
- special internal handling of f/16;
- independent JNI evidence that `blur_strength` is an active bounded process
  parameter, not display-only metadata.

Still requiring a device hook or further type recovery:

- authoritative C names for every field in the 768-byte package header;
- exact boundaries/names of all later auxiliary buffers;
- the exact mathematical contribution of `blur_strength` inside the current
  X9 Ultra OPLUS backend after APS parsing (the fallback ArcSoft backend is
  confirmed to consume it, but its formula is not transferable);
- every coefficient selected by zoom/scene from the native calibration state;
- a direct numerical mapping from the OPPO curve to Apple's private REND
  renderer.

The next high-value hook is at `bokehSetBokehEffectPara` after
`ZSTD_decompress`, plus `GetBlurmapEngine::convertDepthToRadius`. Logging the
parsed pointers, selected `focusDepth`, `bgCoef`, `fgCoef`, `tblVal`, incoming
and used f-number on one real 23x/70x/139x/230x matrix would close the remaining
render-model gap without guessing from output images alone.

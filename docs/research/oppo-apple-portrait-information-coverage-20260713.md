# OPPO-to-Apple Portrait Information-Coverage Audit

Date: 2026-07-13

## Verdict

The current converter is a strong Apple-consumable interoperability MVP, but
it does **not** yet use every piece of OPPO portrait information. The safe
claim is:

- all 80 deduplicated camera originals produce an ISO gain map, Float16
  disparity, Portrait Effects Matte and Apple portrait metadata offline;
- the high-value OPPO inputs with known Apple equivalents are mapped;
- OPPO's complete semantic package and nonlinear bokeh model are not yet
  reproduced, so blur and edge fidelity remain experimental.

Device import/editing across the 80-file matrix remains the final acceptance
gate. Offline auxiliary visibility is necessary but not sufficient.

## Batch evidence

Source corpus: 80 deduplicated `IMG*.heic` camera originals from the local
OPPO import directory.

| Gate | Result |
|---|---:|
| Converted outputs | 80/80 |
| ISO gain map visible to ImageIO | 80/80 |
| Float16 disparity visible to ImageIO | 80/80 |
| Portrait Effects Matte visible to ImageIO | 80/80 |
| Semantic hair matte | 44/80, exactly when a usable OPPO hair plane exists |
| Orientation | 60 Orientation 6; 20 Orientation 1 |
| Source total | 483.46 MiB |
| Apple-output total | 98.76 MiB |
| Output/source ratio | 20.4% |

The size reduction is intentional: Apple portrait output does not append the
large OPPO private portrait tail after converting its useful content.

Two originals lacked `local.uhdr.gainmap.info` but already contained a valid
outer ISO gain-map graph. Their 20-float gain metadata is reconstructed from
`HDRToneMap` gain min/max, gamma, offsets and headroom instead of using a
synthetic default. One of those also lacked the UserComment portrait routing
bit; explicit `--apple-portrait` recovers it from the authoritative
`rear.depth + rear.depth.config + src.image` resource set and emits a warning.

## Mapping coverage

| OPPO source | Apple output | Status |
|---|---|---|
| first `src.image` JPEG | base HEVC payload | mapped; encoded once |
| second `src.image` JPEG | ISO gain-map HEVC payload | mapped; encoded once |
| private 80-byte gain info or outer `HDRToneMap` | ISO gain-map/tmap metadata | mapped |
| rank plane | relative Float16 disparity | mapped |
| header width/height and rank scale | disparity geometry/amplitude | mapped |
| header min/max quantization endpoints and exponent | reconstruct OPPO internal absolute focus-disparity feature | decoded; pending Apple REND fit |
| continuous header fx | source-depth geometry diagnostic and Apple profile/crop selection | mapped without multiplying disparity a second time |
| physical/equivalent focal length and zoom | primary EXIF capture identity | retained |
| portrait plane | Portrait Effects Matte topology | mapped when nonzero |
| pet plane | merged into Portrait Effects Matte | mapped; Apple has no matching pet auxiliary type |
| hair plane | PEM refinement plus semantic hair matte | mapped when nonzero |
| config focus point | Apple Focus XMP plus local subject-gated focus-rank anchor | mapped in raw `src.image` coordinates |
| config distance (v4 offset 296) | Apple MakerNote `AFMeasuredDepth` (tag 56) | semantic direct transfer; matched captures preserve ordering but do not prove identical cross-device scale |
| config f-number | initial Apple simulated aperture | mapped |
| outer EXIF/GPS/date/orientation | primary metadata and geometry | retained |
| face-attention/saliency analysis | Apple Focus XMP | generated fallback, not an OPPO-field translation |

Across the originals, all 80 set the portrait-plane flag, but only 51 contain
a nonzero portrait plane. Two contain a nonzero pet plane. The OPPO subject
topology is therefore available for 53/80; the other 27 use Vision person
segmentation as the PEM fallback. A nonzero hair plane exists in 44/80.

## Information not yet mapped

The following gaps prevent a claim of complete OPPO-to-Apple semantic
conversion:

1. `rear.depth.config` carries a valid focus rect, distance, `blurStrength`,
   the 22-point aperture curve, foreground blur scale, scene mode, face
   rectangles and 296 keypoints per face. The current output uses the focus
   point and f-number and maps distance to Apple's named `AFMeasuredDepth`
   field, but it does not yet reproduce OPPO's native face/portrait/pet/
   near-object focus-depth selector or fit the distance prior into private
   REND scene coefficients.
2. Producer-side firmware now explains most of the 4.10-7.31 MiB after rank
   and same-size hair/portrait/pet planes: semantic segmentation, variable
   motion/spotlight blocks, a rectified master/slave NV21 pair and an optional
   model-output image. Static evidence does not identify that optional image as
   a rendered-bokeh frame; `IMG20260506112827` contains only the rectified pair,
   while its outer HEIC primary is the final rendered portrait. These blocks
   are camera-algorithm inputs/outputs, not one hidden high-resolution depth
   map. The Apple graph intentionally omits the YUV frames; semantic labels
   remain useful only after their class IDs and confidence semantics are proved.
3. `crop.region` occurs in 73/80 originals and `mesh.coord/config` in 12/80.
   They are not currently used for depth/matte registration. Orientation and
   current samples pass, but crop/mesh-aware registration is still a required
   adversarial test.
4. OPPO's zoom-dependent foreground/background CoC functions, f/16 special
   handling, PSF model and independent `blur_strength` control cannot be
   represented by merely setting Apple's displayed aperture. The current path
   instead selects a real Apple 1x/2x/3x/5x lens renderer profile and leaves
   OPPO rank deltas in their source header scale.
5. Two Apple `REND` families vary with focus/depth state, not edited aperture.
   In controlled near/middle/far refocus captures, `0x01c3` and `0x01c4` remain
   exact profile-specific multiples of `0x01c2`, but `0x01c5` retains a second
   scene term at 1x and 2x. Separately, `0x0192 = 48 * 0x0191`, while
   `0x0193 / 0x0191` is a profile constant (`1.0` at 1x, `0.4` at 2x/3x);
   `0x0190/0x0191` also move with refocus at 1x/2x. The current implementation
   uses representative profile values. Matched 2x/3x near/far pairs confirm
   that `AFMeasuredDepth` tracks OPPO config distance, but also prove that the
   `01c2...01c5` response reverses direction between 2x and 3x. The principal
   fidelity gap is therefore a profile-specific fit of
   at least two Apple scene controls from OPPO's semantic-selected focus
   disparity, quantization endpoints, focus branch and distance prior, not
   copying config distance into unrelated floats.
6. Watermark/master-mode resources and the complete private OPPO re-edit graph
   are intentionally omitted from Apple portrait output. Users who need OPPO
   re-editability must select OPPO-compatible preservation instead.

## Product modes and size policy

Apple portrait output and OPPO-compatible preservation are mutually exclusive:

- `--apple-portrait`: convert known resources into the Apple graph, omit the
  redundant large OPPO portrait tail, and force OPPO compatibility off;
- `--oppo-compatible` / `--oppo-compat`: retain the OPPO-oriented output path
  and private re-edit resources, without synthesizing a second Apple portrait
  graph;
- no portrait switch: preserve current normal XDRemux behavior.

Explicitly enabling both modes is a CLI error for both `convert` and `batch`.
`--apple-portrait --no-oppo-compat` is accepted but redundant.

## Next quality work

Priority order:

1. device-import the clean 80-file matrix and record Photos recognition,
   initial aperture, refocus and f/1.4/f/16 behavior;
2. implement the native-like OPPO focus selector: focus ROI, face/keypoints,
   portrait/pet component, near-object branch, then histogram fallback;
3. prove crop/mesh registration semantics on the 12 mesh samples;
4. decode semantic label IDs and the variable motion/spotlight block only if
   they materially improve PEM/hair or reject unreliable focus samples;
5. extend the controlled Apple refocus set with measured physical distances,
   then fit the `0x0190...0x0193` blur-state family and the
   `0x01c2...0x01c5` scene-scalar family separately from OPPO focus disparity,
   quantization endpoints, inverse config distance, focus branch and
   near-object confidence;
6. fit the separate aperture response from OPPO's 22-point
   aperture/blur-strength curve and `foregroundBlurScale`.

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
| continuous header fx | continuous render-gain endpoint | mapped as renderer scale |
| physical/equivalent focal length and zoom | primary EXIF capture identity | retained |
| portrait plane | Portrait Effects Matte topology | mapped when nonzero |
| pet plane | merged into Portrait Effects Matte | mapped; Apple has no matching pet auxiliary type |
| hair plane | PEM refinement plus semantic hair matte | mapped when nonzero |
| config f-number | initial simulated aperture and typed REND record | mapped |
| outer EXIF/GPS/date/orientation | primary metadata and geometry | retained |
| face-attention/saliency analysis | Apple Focus XMP | generated fallback, not an OPPO-field translation |

Across the originals, all 80 set the portrait-plane flag, but only 51 contain
a nonzero portrait plane. Two contain a nonzero pet plane. The OPPO subject
topology is therefore available for 53/80; the other 27 use Vision person
segmentation as the PEM fallback. A nonzero hair plane exists in 44/80.

## Information not yet mapped

The following gaps prevent a claim of complete OPPO-to-Apple semantic
conversion:

1. `rear.depth.config` carries focus coordinates/rectangle, distance,
   `blurStrength`, the 22-point aperture curve, foreground blur scale, scene
   mode, mirror/camera roll, face rectangles and 296 keypoints per face. The
   current output uses only f-number; Focus is chosen by Vision.
2. Every decoded depth package still has about 4.10-7.31 MiB after rank and
   same-size hair/portrait/pet planes. Firmware evidence associates those
   buffers with probability/semantic labels, monocular depth, confidence,
   stripe/hollow masks and guide NV21/YUV. Their exact per-buffer layout is not
   yet sufficiently proven to write Apple skin/teeth/glasses or improve
   disparity safely.
3. `crop.region` occurs in 73/80 originals and `mesh.coord/config` in 12/80.
   They are not currently used for depth/matte registration. Orientation and
   current samples pass, but crop/mesh-aware registration is still a required
   adversarial test.
4. OPPO's zoom-dependent foreground/background CoC functions, f/16 special
   handling, PSF model and independent `blur_strength` control cannot be
   represented by merely setting Apple's displayed aperture. Pass D's
   continuous render gain is a bracketed approximation.
5. Physical OPPO baseline/focal calibration is intentionally not copied into
   Apple's private REND graph because device tests proved that combination
   double-amplifies long-focal blur. Exact physical calibration requires a
   matching Apple renderer model that is not available.
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
2. decode config focus/face geometry and compare it with Vision-selected Focus;
3. prove crop/mesh registration semantics on the 12 mesh samples;
4. identify the later confidence/probability/semantic buffers and test whether
   they materially improve PEM/hair or disparity boundaries;
5. fit Apple render gain and aperture response against real device judgments,
   stratified by focal group and distance mode.

# Photographic Styles HDR reconstruction

English | [简体中文](styles-reconstruction.md)

`xdremux reconstruct-styles` reconstructs relative scene luminance from an original Apple HEIF photo. It keeps the compressed SDR primary, its colour properties, and its geometry. A new RGB ISO Gain Map carries the HDR appearance. The SDR appearance stays unchanged.

This command requires macOS, Metal, and the bundled `xdremux-apple-adapter`. It uses private Apple framework methods. The adapter checks their signatures before calling them. A future system version can make this capability unavailable.

## Inputs and outputs

The input must contain Styles metadata linked to the primary, a Style Delta Map, and a Linear Thumbnail. The supported metadata versions are `0.8`, `0.9`, `0.15`, and `0.16`. The command selects resources through HEIF relationships, not fixed item numbers, filenames, or camera models. A Styles name or preset alone is insufficient. Unknown versions, incomplete resources, inconsistent geometry, and failed consumer checks produce an error.

```bash
xdremux reconstruct-styles \
  --input original.HEIC \
  --output-dir reconstructed
```

Repeat `--input` to process a batch. The command continues after an individual failure and returns a JSON result for every input. It exits with status `1` if any input failed.

The default processes native pixels. Use `--max-edge 1024` to limit the processing and Gain Map resolution. This option also limits the diagnostic EXR resolution. It does not resize or re-encode the final HEIC SDR primary.

Each source gets a new `<source-stem>.reconstructed` directory:

| File | Purpose |
| --- | --- |
| `hdr.heic` | Original SDR primary with the reconstructed RGB ISO Gain Map. |
| `scene_linear.exr` | Relative extended-linear Display P3 reconstruction for analysis. |
| `unstyled.exr` | Native Styles reversal before scene-luminance guidance. |
| `report.json` | Source/output hashes, selected resource IDs, native facts, gain limits, and readback checks. |
| `globalToneCurveData.bin` | Native decoded tone-curve nodes, when the native parser supplies them. |
| `lightMapData.bin`, `linearLightMapData.bin` | Native decoded illumination buffers, when available. |

## Separate HDR display target

Use `--display-headroom 2`, `4`, or `8` to render a display target H from the reconstructed scene reference R. This creates `<source-stem>.display-<headroom>x.reconstructed`, adds `display_linear.exr`, and puts H in `hdr.heic`. The default command continues to put R in `hdr.heic`.

```bash
xdremux reconstruct-styles --input original.HEIC --output-dir reconstructed --display-headroom 8
```

The display path separates global range mapping, coarse spatial compensation, and creative chroma/skin protection. It does not expose Apple's Tone/Color editor coordinates or a two-dimensional palette. All requested capacities use the same scene-derived 8x master. A 2x or 4x file contains the corresponding endpoint of that common family, with its own declared capacity.

The analysis uses area luminance at a long edge of up to 512 pixels. It selects a median scene/SDR exposure and fits a monotone global scene-to-SDR luminance curve with robust bin medians. The difference between the 2nd-to-98th percentile scene range and that fitted SDR range is a range-gap proxy. Headroom is a peak/reference-white ratio. This proxy does not measure display black, ambient reflection, absolute scene radiance, or a calibrated physical dynamic range.

A bounded SDR spatial residual remains after the global fit. A joint bilateral average follows scene luminance edges, with spatial sigma 4 pixels and range sigma 0.5 EV. Edge-aware upsampling returns a coarse field C. The residual limit is 1.5 EV. C is a candidate dynamic-range compensation field. It can contain creative or semantic effects and cannot identify the camera's actual operators. Fine detail in R remains; this path adds no sharpening or blanket blur. R can itself retain capture sharpening and local-processing artifacts.

The clean master uses `A = max(R,0) * exposure` and the global rolloff `A * 8 / (8 + Y(A))`. The color stage anchors relative opponent chroma to the same photo's extended-linear SDR. It preserves post-tone luminance and fits the display gamut. Linked 2026 semantic-person and legacy 2019 skin mattes supply color protection weights. If semantic-person is absent, a linked 2018 Portrait Effects Matte can supply an explicitly reported foreground proxy. Missing masks produce zero weights; ambiguous masks fail. Sky and newer skin variants are never substituted. These choices do not reproduce the full Apple forward renderer. The independently callable midtone and skin-brightness leaves remain available to research callers but are disabled in this range family.

The common target family is authored before carrier gain calculation. Let `L(v) = log2(max(v,0) + 1/64)`, applied per RGB channel; `Q` is the clean 8x master, `S` is the original SDR, and `g` is the range-gap proxy in EV. For headroom h from 1 through 8:

```text
w(h) = log2(h) / 3
lambda(h) = 1 - log2(h) / max(3, g)
B = L(S) - C
L(H(h)) = (1 - w(h)) * B + w(h) * L(Q) + lambda(h) * C
```

The 1x target returns the original SDR exactly. Other targets invert L and project to `[0,h]`. Equal offsets and an affine local schedule allow ISO 21496-1:2025 clause 6.3 to carry this family in a single Gain Map before range projection and clipping. The standalone gap schedule `max(1-log2(h)/max(g,0.01),0)` is also reported. When g is below 3 EV, it removes compensation earlier than the single-map schedule. The implementation reports this compromise. It does not claim an arbitrary headroom-dependent renderer in a standard file. For a file with declared capacity c, its carrier weight is `log2(min(h,c))/log2(c)`. The report distinguishes this weight from the common 8x master weight w(h). RGB range projection, negative source channels, sampling and codec losses can prevent exact agreement. When g is at most 3 EV, C cancels algebraically and the family is an offset-log blend of S and Q. The measured residual is added to the HDR master only when the estimated gap exceeds that capacity.

The report records the global curve, gap proxy, residual strength/limits, actual mask roles/URIs/IDs/hashes, exposure, finite float target, range-projected fraction, and native headroom/readback. Before publication, the adapter applies the actual encoded Gain Map from the same file at 1x, 1.5x, 2x, 3x, 4x, 6x and 8x. Requests above the file capacity use its endpoint. Each readback must have mean luminance error at most 0.05 EV against the authored target at that capacity. Negative RGB and nonpositive decoded luminance remain in the error tails. The report includes the p99 and maximum errors; the mean limit is not a per-pixel guarantee.

The declared display capacity is independent of the largest local gain. HEVC quantization can overshoot the float bound and produce near-black negative values. Native readback and unchanged SDR do not establish physical HDR viewing quality. Original texture metadata remains; the texture renderer is not rerun. See [tonal and color methods](research/styles-tonal-color-methods.en.md) for the mechanism and evidence boundaries.

EXR pixels have the displayed orientation. The reconstruction works in storage orientation so that auxiliary data and compressed SDR remain aligned. The EXR writer can use half precision. The report records its measured readback error.

Existing bundles are never replaced. Duplicate source stems in one output directory produce an explicit failure. The report is written last. A failed invocation removes only the new directory it reserved. Source photos are read-only.

## Reconstruction and validation

1. Rust extracts the linked metadata and complete auxiliary image subgraphs. This prevents ImageIO from selecting another primary through an alternative-image group.
2. The adapter asks Apple for reverse configuration and tuning. It checks coefficient storage against the native texture dimensions. It maps the Delta Map to the primary canvas independently of the Delta Map's stored size.
3. Apple's reverse processor supplies the unstyled colours. The original HDR supplies fine luminance detail.
4. Rust decodes the Linear Thumbnail with the measured native exponential inverse, `2^((code - 0.69336945) / 0.08550479) - 0.00964052`. It derives a bounded luminance gain field and stabilizes weak primary structure as described below. Joint bilateral interpolation follows primary luminance edges. Native YCbCr-to-RGB decoding can produce small values outside the nominal code range; those values do not select a different transfer function.
5. A robust scalar preserves the original HDR midtone exposure. No RAW file, per-camera fit, filename-specific parameter, or trained model is used at runtime.
6. Rust derives RGB gain values relative to the original SDR. Negative gain values can darken HDR shadows. A shared `1/64` offset keeps black finite. HEVC 4:4:4 encodes the gain image. The original compressed SDR tree is copied.
7. The writer uses the displayed primary dimensions and identity rotation for the derived `tmap`. Copying a portrait primary's rotation to the derived image makes ImageIO ignore the ISO Gain Map.
8. Before success, the command checks compressed SDR identity, native SDR readback, native HDR expansion, EXR readback, and source integrity. SDR RGB error must not exceed `1e-6`. Mean HDR target luminance error must not exceed `0.15 EV`. Mean EXR luminance roundtrip error must not exceed `0.005 EV`.

The generated HEIC retains source metadata and auxiliary resources. Existing Styles resources describe the original capture. This command does not establish that later interactive Styles editing in Photos remains equivalent after the Gain Map changes.

## Base-guided flat-region stabilization

The thumbnail can contain low-frequency luminance structure that the existing rendering suppressed. Directly transferring every difference can make a smooth surface uneven. The full-resolution linear SDR base supplies the structure guide for constraining this transfer before defining R and the display family. It does not supply the target scene luminance. This is an appearance safeguard, not a classifier of true illumination or capture artifacts.

Analysis uses area-averaged linear luminance at up to 512 pixels on the long edge. A radius-4 neighborhood measures log-luminance variance in the SDR base. Pixels with standard deviation below 0.075 EV form candidate regions, connected through four neighbors whose log-luminance difference is at most 0.1 EV. A region must contain at least 64 pixels and span at most 0.4 EV in base luminance. The neighborhood radius scales with analysis edge, and the minimum area scales with its square. Invalid nodes do not participate.

An accepted region receives the median thumbnail-to-original-HDR log gain after exposure anchoring. At least 90% of candidate samples, and at least the minimum region area, must be within 0.25 EV of this median. Only these inliers form ordinary flat cores; outliers retain their existing guidance unless the thin-feature test below applies. Broad scene differences fail this support requirement. Nearby cores separated by thin text or rules share one gain when they lie within radius 16, their mean base levels differ by at most 0.1 EV, their combined base span is at most 0.4 EV, and their combined gain span is at most 0.5 EV. The radius scales with analysis edge. Their pooled median is projected to the interval that keeps every core sample within the 0.25 EV change bound. This avoids separate brightness plateaus on one surface. The constant gain extends into a compatible boundary band of radius 4: base luminance must lie within the region range plus 0.1 EV. The inner half has full strength; the outer half uses a cubic smoothstep to zero. Its radius scales with analysis edge. Other regions retain the original thumbnail guidance.

Joint bilateral interpolation uses each full-resolution SDR-base pixel as its range guide, with range sigma 0.5 EV and a 4x4 support. It blends the authored regional gain with the original interpolated gain. The normal final change is bounded to 0.25 EV per full-resolution pixel; the thin-feature exception is described below. Defining a constant gain before interpolation prevents the residual thumbnail pattern from surviving as a resampled difference. Original HDR fine luminance and unstyled colour remain the pixel sources; primary RGB is never spatially averaged. This does not guarantee unchanged texture or exclude every halo in real photographs.

Thin text, rules and similar small features require a separate confidence test. A radius-3 median of the SDR-base log luminance supplies a surface guide. A difference above 0.1 EV identifies a candidate small structure; broad steps that survive the median do not trigger it. A nearby accepted flat core must match that surface tone. These radii scale with analysis edge.

A candidate is accepted only if paired linear thumbnail/original-HDR samples support a positive local affine relationship. The primary is tested with box-blur radii 0, 1 and 2 analysis pixels to account for thumbnail blur before estimating contrast. There is no shift search. Each model fits alternating pixels of the local patch and validates the other pixels: at least 90% valid support, positive correlation at least 0.5, a primary variance above `(meanHDR * 0.02)^2`, and held-out RMS log-luminance error at most 0.1 EV. The normalized slope must be in [0.25,4], and its background prediction must agree with the flat core within 0.1 EV. This is the photometric path: only it may author foreground contrast. A separate background-only path requires correlation at least 0.9 and an inferred background consistent with the core within the normal 0.25 EV flat-region budget. This path does not use its uncertain slope on the foreground. It modifies only full-resolution SDR pixels inside the accepted core’s observed tone range or within 0.1 EV of it, with a smooth taper to no correction at a distance of 0.2 EV. Using the core range rather than a stroke-contaminated local median retains the primary’s original background undershoot while removing new gain around it. A candidate failing both paths receives no thin-feature correction. Accepted seeds extend support, with full strength within radius 3 and a smooth transition to zero at radius 6. Candidate, photometric and background-only seed counts are reported.

At each full-resolution pixel let `x = originalHDRY / medianHDRY`, `k` be the core's linear gain, and `c` the relative slope. For `c >= 1`, use `t = smoothstep(clamp((abs(log2(x))-0.15)/0.5,0,1))` and `a = 1+(c-1)*t`; this preserves the original fine background texture in the 0.15 EV band. For `c < 1`, use `a = c` so that a real reduction of foreground contrast is allowed without a nonmonotone knee. The photometric target is `k * medianHDRY * max(1+a*(x-1),1e-6)`, monotone in primary luminance for each fixed model. Background-only evidence leaves foreground gain on its original path and constrains only matching background pixels. Positive joint interpolation mixes these aligned targets in log-gain space. Primary pixels are not spatially averaged; the thumbnail's blurred outline is not copied.

The regression set includes an identity case containing gradients, dense texture and small highlights; a blurred-thumbnail stroke case; genuine HDR foreground contrast both above and below the primary; displaced and reversed structures; dense texture without a flat donor; broad edges; large genuine scene differences; and a background-only invariant that forbids it from authoring foreground tone. These tests bound specific failure modes. They do not establish universal scene classification. In particular, a weak real illumination difference and a thumbnail error can produce the same available observations. Flat-surface regularization is therefore a bounded appearance prior, not proof that thumbnail content is erroneous. No physical scene-recovery guarantee follows from a clearer SDR base or from passing these fixtures.

The normal change limit remains 0.25 EV. Interpolated thin-feature support raises it toward 1.5 EV only where the low-resolution ratio is unreliable. This larger bound can remove the spurious edge gains that exceed the flat-surface budget. It is not a universal text detector or a recovery of the original optical point-spread function; existing source sharpening and colour fringes can remain.

The report records the guide, analysis geometry, policy, accepted-region count/fraction, thin-feature fraction, and actual full-resolution adjustment RMS/maximum. Both the default scene outcome and the separate display outcome use this stabilized R. The original SDR remains unchanged. This deliberately trades some agreement with the thumbnail or RAW for stable flat surfaces; RAW agreement alone cannot decide the intended display appearance. It does not remove every inherited local-processing artifact or recover a calibrated physical scene.

## Limits

The result approximates relative scene luminance. It does not recover RAW data or prove inversion of the complete capture ISP. The embedded thumbnail has limited resolution and precision. Clipped colours and detail absent from the primary cannot be recovered. Negative linear colour channels cannot be represented exactly by the positive HDR Gain Map and are clipped for that output; the report records their fraction.

The tone curve and two illumination buffers are decoded for inspection. Their decoding does not establish the complete capture tone-mapping composition. The runtime does not divide the two illumination buffers and label the result an inverse ISP.

The current path uses the measured native exponential inverse through the toe. It does not establish the exact capture transfer for every historical Styles file. Unknown metadata versions fail closed. Capability checks and readback checks do not replace RAW-reference validation on additional scenes.

Absolute radiance and diffuse white are not calibrated. In particular, this command does not define diffuse white as 203 nits. HDR display adaptation and real Photos/device appearance require separate consumer evidence.

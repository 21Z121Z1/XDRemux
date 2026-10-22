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

The display path uses independent midtone, skin-brightness and opponent-chroma methods. It does not expose Apple's Tone/Color editor coordinates or a two-dimensional palette. It selects one global exposure from positive R/SDR samples, applies the declared rolloff `R * exposure * h / (h + Y(R * exposure))`, and normalizes by `h` before bounded leaves. R is not clipped to the editor's nominal range. The color stage anchors relative opponent chroma to the same photo's extended-linear SDR, preserves the post-tone luminance, and fits the display gamut at that luminance. These are explicit pipeline choices, not a claim of complete Apple forward-renderer equivalence.

Linked 2026 semantic-person and legacy 2019 skin mattes supply weights. If semantic-person is absent, a linked 2018 Portrait Effects Matte may supply an explicitly reported foreground proxy. Missing masks produce zero weights; ambiguous masks fail. Sky and newer skin variants are never silently substituted. The adapter decodes these masks without color management or orientation, in the primary's storage geometry. The guide is this implementation's area-luminance/log-Gaussian guide, not a reproduction of Apple's statistics or guided filter.

The report records the coefficients, actual mask roles/URIs/IDs/hashes, exposure, finite bounded float target, and native headroom/readback. The declared display capacity is independent of the largest local gain. HEVC quantization can overshoot the float bound, introduce near-black negative values, and change intermediate-headroom luminance. Native readback and exact SDR preservation do not establish physical HDR viewing quality. Source texture/detail and original texture metadata remain; the texture renderer is not rerun. See [tonal and color methods](research/styles-tonal-color-methods.en.md) for the mechanism and evidence boundaries.

EXR pixels have the displayed orientation. The reconstruction works in storage orientation so that auxiliary data and compressed SDR remain aligned. The EXR writer can use half precision. The report records its measured readback error.

Existing bundles are never replaced. Duplicate source stems in one output directory produce an explicit failure. The report is written last. A failed invocation removes only the new directory it reserved. Source photos are read-only.

## Reconstruction and validation

1. Rust extracts the linked metadata and complete auxiliary image subgraphs. This prevents ImageIO from selecting another primary through an alternative-image group.
2. The adapter asks Apple for reverse configuration and tuning. It checks coefficient storage against the native texture dimensions. It maps the Delta Map to the primary canvas independently of the Delta Map's stored size.
3. Apple's reverse processor supplies the unstyled colours. The original HDR supplies fine luminance detail.
4. Rust decodes the Linear Thumbnail with the measured native exponential inverse, `2^((code - 0.69336945) / 0.08550479) - 0.00964052`. It derives a bounded luminance gain field. Joint bilateral interpolation follows primary luminance edges. Native YCbCr-to-RGB decoding can produce small values outside the nominal code range; those values do not select a different transfer function.
5. A robust scalar preserves the original HDR midtone exposure. No RAW file, per-camera fit, filename-specific parameter, or trained model is used at runtime.
6. Rust derives RGB gain values relative to the original SDR. Negative gain values can darken HDR shadows. A shared `1/64` offset keeps black finite. HEVC 4:4:4 encodes the gain image. The original compressed SDR tree is copied.
7. The writer uses the displayed primary dimensions and identity rotation for the derived `tmap`. Copying a portrait primary's rotation to the derived image makes ImageIO ignore the ISO Gain Map.
8. Before success, the command checks compressed SDR identity, native SDR readback, native HDR expansion, EXR readback, and source integrity. SDR RGB error must not exceed `1e-6`. Mean HDR target luminance error must not exceed `0.15 EV`. Mean EXR luminance roundtrip error must not exceed `0.005 EV`.

The generated HEIC retains source metadata and auxiliary resources. Existing Styles resources describe the original capture. This command does not establish that later interactive Styles editing in Photos remains equivalent after the Gain Map changes.

## Limits

The result approximates relative scene luminance. It does not recover RAW data or prove inversion of the complete capture ISP. The embedded thumbnail has limited resolution and precision. Clipped colours and detail absent from the primary cannot be recovered. Negative linear colour channels cannot be represented exactly by the positive HDR Gain Map and are clipped for that output; the report records their fraction.

The tone curve and two illumination buffers are decoded for inspection. Their decoding does not establish the complete capture tone-mapping composition. The runtime does not divide the two illumination buffers and label the result an inverse ISP.

The current path uses the measured native exponential inverse through the toe. It does not establish the exact capture transfer for every historical Styles file. Unknown metadata versions fail closed. Capability checks and readback checks do not replace RAW-reference validation on additional scenes.

Absolute radiance and diffuse white are not calibrated. In particular, this command does not define diffuse white as 203 nits. HDR display adaptation and real Photos/device appearance require separate consumer evidence.

# Apple Photographic Styles and Portrait

English | [简体中文](apple-features.md)

In addition to standard HDR output, XDRemux can generate Photographic Styles or portrait-editing resources for Apple Photos. Both features are opt-in and remain experimental.

## Feature scope

| Feature | User result |
| --- | --- |
| Apple Photographic Styles | Switch Photographic Styles and adjust tone, color, and intensity in Apple Photos |
| Apple Portrait | Adjust simulated aperture and, when supported, select a new focus point in Apple Photos |
| Combined mode | Keep HDR, Photographic Styles, and portrait editing in one final HEIC |

These features process only the current input photo. They do not copy image or editing resources from another photo.

## Requirements

- macOS 15 or later.
- From a source checkout, run one complete `swift build` without restricting the product.
- Apple Portrait requires `zstd` on `PATH`.
- JPEG portrait bridging also requires `ultrahdr_app` on `PATH`.
- The current system must provide the required Apple image-analysis services.

If a required system capability is unavailable, conversion returns an explicit error instead of writing fabricated empty resources.

## Apple Photographic Styles

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux derives the semantic regions and Photographic Styles resources required by Apple Photos from the current image. Only valid detected regions are written; small but valid regions are retained.

This mode continues to produce standard HDR output. It cannot be combined with `--oppo-compatible`.

## Apple Portrait

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

The source must contain recoverable vendor depth, focus, and unblurred-image resources. XDRemux converts them into portrait resources that Apple Photos can continue editing while preserving the saved focus and simulated aperture when possible.

Portrait conversion is unavailable for an ordinary non-portrait photo without the required depth resources. Portrait-only batches report such an input as failed. In a combined Styles and Portrait batch, the same input may continue as Styles-only with a warning.

Every successful portrait output also writes `<output>.portrait-manifest.json`, which records input capabilities, conversion choices, warnings, and reviewable validation information.

## JPEG portrait bridge

Apple Portrait also accepts OPPO HDR JPEG inputs that contain a standard HDR Gain Map and complete vendor portrait resources. Select JPEG explicitly in batch mode:

```bash
swift run xdremux batch \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

JPEG input is accepted only with `--apple-portrait`, optionally together with Photographic Styles. Standard HDR, Styles-only, and OPPO-compatible modes continue to use HEIC input.

## Combination and conflicts

Photographic Styles and Portrait can be enabled together:

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

Combined mode writes one final HEIC. `--oppo-compatible` targets OPPO Gallery while Apple modes target Apple Photos, so they cannot be combined. The CLI rejects the conflicting options before writing output.

## Current validation status

The implementation currently covers these offline checks:

- Output containers can be reopened.
- Standard HDR Gain Map and Apple auxiliary references can be parsed.
- Photographic Styles and Portrait resources pass the repository validators.
- The App and CLI produce identical output for the same conversion request.
- Existing real samples have passed limited macOS Photos editing, refocus, and save/reopen checks.

These results do not qualify every device, focal length, operating-system release, or Apple Photos version. Offline structural validation is not equivalent to real display behavior on an iPhone, Mac, or in OPPO Gallery.

See the [technical implementation index](xdremux/README.md), [ISO container audit](xdremux/iso-conformance-audit-20260511.md), and [validation guide](validation/README.md). Per-sample logs, firmware fields, and reverse-engineering evidence remain research material rather than product commitments.

# Apple Features

English | [简体中文](apple-features.md)

XDRemux has Apple-specific conversion paths for Photographic Styles, Apple Portrait, and Apple Live Photo metadata.

These paths are separate from the standard ISO HDR path. Some combinations are supported, and some combinations are rejected before output is written.

## Platform boundary

The Swift package requires macOS 15 or later.

Apple-specific analysis and rendering use Apple platform frameworks and helper processes. The normal cross-platform Python converter does not generate Photographic Styles or Apple Portrait data.

OPPO-compatible HDR output and Apple-specific editing output are mutually exclusive.

## Photographic Styles

Enable Photographic Styles with:

```bash
xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

When Photographic Styles is enabled and no producer is selected, the CLI uses `constrained-solver`.

The parser also accepts:

- `--apple-style-data-producer constrained-solver`
- `--apple-style-data-producer learn-node`
- `--apple-style-data-producer identity-fallback`
- `--apple-styles-raw-dng <file>`

The producer option and RAW DNG option require `--apple-photographic-styles`.

`learn-node` and `identity-fallback` are diagnostic or research controls. Do not treat them as the normal product default.

A supplied RAW DNG must match the source photo requirements enforced by the pipeline. The conversion rejects an unusable optional RAW input instead of silently applying unrelated RAW data.

### Validation boundary

The Photographic Styles pipeline validates its generated HEIC structure and its Apple style resources with repository validators and native Apple components where the host supports them.

Private Apple interfaces can change between macOS releases. The repository includes runtime ABI checks for the private selectors used by the style-response tools. Unsupported private ABI shapes fail with a compatibility error instead of being invoked with an assumed function signature.

Offline structural validation is not the same as a complete import-edit-save-reopen test on every Apple Photos version. Treat a device-dependent editing claim as device-dependent evidence.

## Apple Portrait

Enable Apple Portrait with:

```bash
xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

The source must contain the portrait resources required by the conversion pipeline. A normal non-portrait photo does not automatically become a portrait photo.

The conversion can use source depth, focus, aperture, semantic, and restore-original resources when they are present and valid.

Supported portrait JPEG inputs are accepted only through the Apple Portrait path. The output is HEIC.

A successful portrait conversion can emit a portrait manifest next to the output. The manifest records the resources and decisions used by the conversion. It is diagnostic data and is not imported into Apple Photos.

## Photographic Styles + Apple Portrait

Static-photo conversion can enable both options:

```bash
xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

When Styles is enabled, the Apple feature engine runs the Photographic Styles pipeline. The Styles pipeline owns the combined Styles + Portrait output contract.

If a source does not contain the required portrait data, do not assume that a combined request can produce valid portrait editing data.

## Motion Photo + Photographic Styles

The current Swift CLI has a separate single-file bridge for this combination:

```bash
xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.jpg \
  --output IMG_001_apple.heic
```

This path first converts the Motion Photo to an Apple Live Photo pair. It then generates Photographic Styles on the Live Photo still and verifies that the Live Photo asset identifier remains valid.

This combination does not support Apple Portrait in the same pass.

This combination is not the same as plain Motion Photo conversion. The hosted style-rich path does not use PhotoKit loading as a write gate because some hosted macOS versions do not complete that display-object request for external style-rich HEIC resources. The deterministic Live Photo validator and the Photographic Styles validator remain required before publication.

Plain Motion Photo conversion continues to use its own Live Photo validation path.

## Unsupported combinations

The CLI rejects these combinations:

- Apple features + OPPO-compatible output.
- Plain Motion Photo + Apple Portrait.
- Motion Photo + Photographic Styles + Apple Portrait.
- Style producer selection without `--apple-photographic-styles`.
- Styles RAW DNG input without `--apple-photographic-styles`.

## Research controls

The repository contains environment variables and optional model paths for style research.

A research control can change solver behavior or validation scope. Do not describe a research result as the default product result unless the default code path uses the same configuration.

The optional `ReverseKey1Ensemble` model is documented in the [model card](../Models/ReverseKey1Ensemble.model-card.en.md).

## Acceptance

Use three separate evidence classes:

1. Structural evidence proves that the HEIF resources and metadata are present and parseable.
2. Native framework evidence proves that the tested macOS framework accepts the generated resources.
3. Device evidence proves behavior in a specific Apple Photos version on a real device.

Do not replace device evidence with structural evidence when the product claim is about interactive Apple Photos editing.

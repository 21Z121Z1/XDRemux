# Apple Photographic Styles and Portrait

English | [简体中文](apple-features.md)

Beyond the standard HDR output, XDRemux can generate the data that makes a photo editable in Apple Photos. Both features are off by default and both are experimental.

| Feature | What you get |
| --- | --- |
| Apple Photographic Styles | Switch styles in Apple Photos and adjust tone, colour, and intensity |
| Apple portrait | Adjust the blur in Apple Photos, and refocus where the data allows |
| Both together | HDR, style editing, and portrait editing preserved in one file |

Everything is computed from your own photo. Nothing is copied from another picture — not the image, not the editing parameters.

## Requirements

- macOS 15 or newer
- Run a full `swift build` once when working from source
- `zstd` for Apple portrait (`brew install zstd`)
- `ultrahdr_app` as well, for JPEG portraits
- The system must provide Apple's image-analysis capability

When a capability is missing the conversion fails with a clear error rather than writing an empty placeholder.

## Apple Photographic Styles

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

XDRemux analyses the photo's content, brightness, colour, and regions such as people and sky, and generates the data Photographic Styles needs. Only regions actually detected are written.

The resulting style parameters are checked against how a native iPhone photo responds in the editor. If this photo's response falls outside the range of the native samples, that gets folded into what the solver is optimizing for, and the result is required to be no worse than before the correction. Photos that are already in range take a fast path with a single check at the end.

This mode keeps the standard HDR output. It cannot be combined with `--oppo-compatible`.

Solving is CPU-heavy. For batches, use a release build: `swift build -c release`, then run `.build/release/xdremux`.

## Apple portrait

```bash
swift run xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

The source photo must have been shot in portrait mode, with its depth data, focus information, and un-blurred original still present in the file. XDRemux converts those into the form Apple Photos can keep editing, preserving the original focus point and blur strength as far as possible.

Ordinary non-portrait photos do not carry that data, so portrait conversion is unavailable for them. A portrait-only batch records them as failures; if Photographic Styles is also enabled, such a photo falls back to styles-only output.

Each successful portrait conversion also writes `<output>.portrait-manifest.json` next to the result, recording what the input carried, what the conversion chose, and any warnings. That file does not need to be imported into Apple Photos.

## JPEG portraits

Some OPPO portrait photos are JPEG on the outside. Those are accepted only with `--apple-portrait`, and the output is still HEIC. Batches need it spelled out:

```bash
swift run xdremux batch \
  --apple-portrait \
  --glob '*.jpg' \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

Every other mode — standard HDR, styles alone, OPPO-compatible — still takes HEIC only.

## Enabling both

```bash
swift run xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

This produces a single file. `--oppo-compatible` targets OPPO Gallery and the Apple modes target Apple Photos; they cannot be combined, and the CLI rejects that combination before writing anything.

## How far this has been verified

What has been checked:

- The output file reopens.
- The HDR data and the references to the Apple auxiliary resources parse correctly.
- The styles and portrait resources pass the repository's own checking tools.
- The app and the CLI produce the same result for the same request.

What has **not**: importing the file into Apple Photos on a real device, editing it, saving, quitting, and reopening it to confirm the editing still works. That round trip is manual today and is not claimed as a public pass.

That is why the output manifest for these features always records them as not production-ready. Structural checks offline are also not the same as how the photo actually looks on an iPhone, on a Mac, or in OPPO Gallery, and nothing here claims coverage across every model, focal length, or OS version.

## Research switches

Several `XDREMUX_RESEARCH_*` and `XDREMUX_STYLES_*` environment variables select experimental solver paths. **None of them need to be set.** Setting one marks the output manifest as a research run and excludes it from production judgement. They are listed in the [development guide](development.en.md).

Implementation and acceptance material: the [technical index](xdremux/README.en.md), the [ISO container audit](xdremux/iso-conformance-audit-20260511.md), and the [validation guide](validation/README.md).

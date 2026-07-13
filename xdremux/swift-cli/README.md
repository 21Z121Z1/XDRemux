# Swift CLI

This directory contains the Swift command-line converter.

Run it from the repository root:

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

## Apple portrait conversion

OPPO portrait conversion is opt-in:

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic
```

The switch requires both the portrait flag in `UserComment` and a
`rear.depth` tail resource. XDRemux then extracts `src.image`, its gain-map
info, and the zstd-compressed depth automatically; generates a Vision person
segmentation Portrait Effects Matte and face-attention Focus region; and writes
the Apple portrait interoperability metadata.

The `src.image` base and gain map are encoded once. The final container reuses
those first-assembly HEVC payloads byte-for-byte after the auxiliary images are
authored. The CLI requires `zstd` on `PATH` to decode OPPO `rear.depth`.

Camera calibration is derived per capture from OPPO physical focal length,
the optical equivalent focal-length anchor in `LensModel`,
`DigitalZoomRatio`, and `src.image` dimensions. The resulting intrinsic matrix
follows the same shrinking-reference-window representation observed in iPhone
continuous zoom captures. Lens distortion is written as zero under the
rectified-source assumption until real OPPO distortion tables are available.

The Apple simulated aperture is taken from the OPPO portrait edit state in
`rear.depth.config` when available, then from EXIF `FNumber`; `f/1.4` is only a
last-resort compatibility fallback. The resolved value is written as
`depthBlurEffect:SimulatedAperture`.

Without `--apple-portrait`, XDRemux uses its normal gain-map conversion path
and reattaches the original OPPO portrait private tail byte-for-byte. It does
not synthesize Apple depth, matte, Focus, or portrait metadata.

Do not place macOS app project files here; app shells belong under `apps/macos/`.

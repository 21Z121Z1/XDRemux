# XDRemux OPPO Portrait Conversion Plan

Date: 2026-07-13

## Goal

Add a supported, opt-in XDRemux CLI path that detects and converts the OPPO
portrait resources embedded in the input file:

- `src.image` (base JPEG followed by its gain-map JPEG);
- the matching 80-byte gain-map info field;
- zstd-compressed `rear.depth`, decoded automatically;
- an embedded metadata-only Apple compatibility profile supplying the currently
  required MakerNote and portrait-rendering metadata.

The pipeline must generate a Vision person matte and a face-attention Focus
region, preserve OPPO capture EXIF/GPS, and produce the validated Apple-facing
portrait container contract.

## Encoding Boundary

The real `src.image` base and gain map may be encoded only during the initial
`src.image -> ISO gain-map HEIC` assembly. Later stages may encode a blank
scaffold and auxiliary images, but the final base and gain-map HEVC payloads
must be transplanted byte-for-byte from that first assembly.

## CLI Contract

`convert --apple-portrait --input INPUT.heic --output OUTPUT.heic` enables the
Apple portrait path. XDRemux requires both the OPPO portrait bit in
`UserComment` and the `rear.depth` tail resource, then automatically extracts
`src.image`, `local.uhdr.gainmap.info`, and `rear.depth`.

Without `--apple-portrait`, the existing gain-map conversion path remains in
use and the original OPPO portrait private tail is reattached byte-for-byte;
no Apple portrait resources are synthesized. With the switch enabled, missing
portrait signals are an error rather than a silent fallback.

Focus defaults to Vision face detection ranked by attention saliency and falls
back to the attention centroid when no face is found. Orientation is selected
automatically from the outer primary geometry and the first JPEG in
`src.image`: when their width/height are swapped, the JPEG rotation wins;
otherwise the outer orientation wins. Displayed Vision coordinates are mapped
back to stored-image XMP coordinates automatically. The private depth payload
uses standard zstd; the CLI reports a clear dependency error when `zstd` is
absent.

## Acceptance

1. Swift CLI type-checks from the production `xdremux/swift-cli` path.
2. A real OPPO sample produces an ISO-parser PASS output containing disparity,
   ISO gain map, and Portrait Effects Matte item graphs.
3. Every final base and gain-map tile payload is byte-identical to the initial
   assembly.
4. Only intended CLI/docs/tests are committed from a clean `origin/main`
   worktree.
5. The branch is pushed and a draft pull request is opened.

The post-change default-tail reattachment path type-checks, but its second
real-sample execution was blocked by the current local execution quota. A
byte-for-byte tail check remains a targeted PR follow-up.

## Device Boundary

The existing manually assembled payload-preserved sample passed iOS
consumption. The integrated CLI output still requires a fresh-device import to
confirm Focus placement and Portrait Effects Matte behavior across arbitrary
inputs.

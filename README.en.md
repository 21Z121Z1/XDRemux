# XDRemux

English Version | [中文版](README.md)

XDRemux converts ProXDR HEIC photos captured on OPPO, OnePlus, and realme devices into standard HDR HEIC files.

It reads the private HDR Gain Map and metadata from the original photo, then repackages them into an HDR HEIC file compliant with ISO 21496-1. The converted photo can be viewed on macOS, iOS, Android, and other systems that support HDR photo display.

## When do I need this tool?

Use XDRemux if you captured ProXDR HEIC photos on an OPPO, OnePlus, or realme phone and want them to keep displaying as HDR photos in other systems or software.

## Output capabilities

The Swift CLI has three opt-in switches. Apple Photographic Styles and Apple
Portrait are independent, disabled by default, and may be written into one
combined HEIC. `--oppo-compatible` remains mutually exclusive with either
Apple output. With no switches, XDRemux uses the standard ISO default.

| Mode | Switch | Result |
|---|---|---|
| Standard ISO (default) | none | ISO 21496-1 HDR with the source Base Image, channel structure, and non-HDR OPPO/QTI metadata tail; Gain Maps may retain HEVC RExt 4:4:4 when the source supports it |
| OPPO Gallery compatible | `--oppo-compatible` | Main Still Picture 4:2:0 Gain Map for OPPO Gallery, with the OPPO private metadata tail preserved |
| Apple Photographic Styles | `--apple-photographic-styles` | Derives StyleEngine coefficients, Linear Thumbnail, Style Delta, per-image statistics, and native-tier semantic auxiliaries from the current input without Apple donor scene payloads |
| Apple portrait | `--apple-portrait` | Converts OPPO depth/aperture into Apple disparity and Focus; Vision supplies high-resolution person/skin/hair/teeth/glasses while OPPO person/hair planes act as constrained topology priors |

> [!IMPORTANT]
> Omitting `--output` or `--output-dir` overwrites inputs. Back up originals
> before conversion.

### Default: standard ISO HDR

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --input IMG_001.heic \
  --output IMG_001_iso.heic

swift xdremux/swift-cli/XDRemux.swift batch \
  --input-dir photo_dump/ \
  --output-dir iso_output/
```

The default does not enable the OPPO-specific compatibility layer. XDRemux
preserves the original Base Image where possible and rebuilds a standard ISO
Gain Map graph. Monochrome sources remain monochrome, while un-downsampled
three-channel sources can retain HEVC Range Extensions 4:4:4. An existing
4:2:0 Gain Map is never advertised as 4:4:4 because discarded chroma cannot be
recovered.

The default preserves non-HDR OPPO/QTI/FileExtendedContainer metadata,
including watermark, master-mode, capture, portrait-editing, and unknown
vendor entries. Private HDR entries under `local.uhdr.*`, `local.hdr.*`,
`src.local.hdr.*`, and `hdr.*` are physically removed, leaving the standard
ISO Gain Map graph as the active HDR display graph.

### `--oppo-compatible`: OPPO Gallery compatibility

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --oppo-compatible \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic
```

This mode converts a high-spec Gain Map to Main Still Picture 4:2:0 so OPPO
Gallery can trigger HDR display. It retains the OPPO private metadata tail for
photos intended to return to the OPPO ecosystem.

### `--apple-photographic-styles`: Apple Photographic Styles

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

Styles runs Apple LearnProcessor on the same input to produce the 51,840-byte
Float16 StyleEngine coefficient buffer. Linear Thumbnail, statistics, and
light maps come from the coherent Base/Gain Map bundle. The verified
`iPhone18,1 / 23F84` profile uses a neutral 0.5 Style Delta; unknown profiles
do not inherit that tuning.

Semantic auxiliaries follow the native tiers: sky only without a credible
person; PEM+skin+sky for a person when Apple Portrait is disabled; no empty
hair/teeth/glasses are added merely to complete a graph. A valid sparse Vision
result is not removed solely because its coverage is small, while unavailable
SPI is a capability error rather than a fake black mask.

### `--apple-portrait`: convert OPPO portrait depth

```bash
swift xdremux/swift-cli/XDRemux.swift convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple_portrait.heic

swift xdremux/swift-cli/XDRemux.swift batch \
  --apple-portrait \
  --input-dir photo_dump/ \
  --output-dir apple_portraits/
```

XDRemux reads `src.image`, `rear.depth`, `rear.depth.config`, and Gain Map
parameters. Base and gain payloads are encoded once and rank becomes Apple
Float16 disparity. Vision is the high-resolution producer for
person/skin/hair/teeth/glasses. Geometry-aligned OPPO portrait/pet data may
supplement only overlapping Vision person topology, while OPPO hair may add
support only inside the final person matte. OPPO data is never reused as
skin, teeth, or glasses.

With both Apple switches, one combined HEIC contains the Portrait family
(PEM+skin+hair+teeth+glasses) plus the Styles sky matte. HDR Base/Gain Map
payloads remain single-copy. In batch mode, a non-portrait input still emits
a styles-only output when Portrait is unavailable.

Apple portrait mode omits the large OPPO portrait tail and private HDR tail
after semantic migration, avoiding duplicate depth or HDR resource sets. Any
Apple output is mutually exclusive with `--oppo-compatible`; enabling both
fails before writing. Blur-strength mapping is still under device validation.

### Python CLI

> [!NOTE]
> Install dependencies first: `pip install pillow-heif Pillow numpy`

```bash
# Single file
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic

# Batch conversion
python3 xdremux/python/XDRemux.py batch --input-dir photo_dump/

# OPPO Gallery-compatible output (--oppo-compat remains as a legacy alias)
python3 xdremux/python/XDRemux.py convert --oppo-compatible --input IMG_001.heic
```

Apple Photographic Styles and Apple Portrait are currently implemented by the
Swift/macOS path. The Python CLI retains its existing HDR conversion support.

### macOS App

Source path:

```text
apps/macos/XDRemuxApp/
```

Build and run locally:

```bash
scripts/build_and_run.sh run
```

## Swift CLI input processing modes

The Swift CLI supports the `--input-processing` option. Most users do not need to set it manually.

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic --input-processing hybrid
```

| Mode | Description |
| --- | --- |
| `hybrid` | Default mode. Preserves the original Base Image and only reprocesses the HDR Gain Map. Non-OPPO outputs keep the original channel layout; OPPO-compatible LHDR uses the verified RGB-copy Gain Map. |
| `system` | Lets the system ImageIO writer produce the final HEIC. This mode re-encodes both the Base Image and the Gain Map, and is useful as a reference for system behavior. |
| `passthrough` | Experimental mode. Rewrites the internal HEIC structure directly for validation and development. Not recommended for normal use. |

## Supported devices

XDRemux is intended for OPPO, OnePlus, and realme devices that can capture ProXDR photos.

The following mainland China models are known to support ProXDR photo capture:

| Brand/series | Models |
| --- | --- |
| OnePlus | OnePlus Ace2 Pro, OnePlus 12, OnePlus Ace3, OnePlus Ace 3V, OnePlus Ace 3 Pro, OnePlus 13, OnePlus Ace 5 series, OnePlus 13T, OnePlus Ace 6, OnePlus Ace 6T, OnePlus Turbo 6, OnePlus 15, OnePlus 15T, OnePlus Ace 5 Supreme Edition |
| OPPO K series | K12, K12x, K13 Turbo series, K15 Pro series |
| OPPO Find series | Find X6, Find X6 Pro, Find N3, Find N3 Flip, Find X7, Find X7 Ultra, Find X8 series, Find N5, Find X8s, Find X9 series, Find N6 |
| OPPO Reno series | Reno10 Pro, Reno10 Pro+, Reno11 Pro, Reno12 series, Reno13 series, Reno14 series, Reno15 series, Reno 16 series |
| realme GT series | realme GT5 series, realme GT5 Pro, realme GT6, realme GT7 Pro, realme GT7 Pro Racing Edition, realme GT7, realme Neo7 Turbo, realme GT8, realme GT8 Pro |
| realme Neo series | realme GT Neo6 SE, realme GT Neo6, realme Neo7, realme Neo7 SE, realme Neo7x, realme Neo8 |
| realme number series | realme 12 Pro, realme 12 Pro+, realme 13 Pro+, realme 13 Pro Supreme Edition, realme 13 Pro, realme 14 Pro+, realme 14 Pro, realme 14, realme 15, realme 15 Pro |

Among them, OPPO Find X8 Ultra, the Find X9 series, and realme GT8 Pro in Ricoh mode support **YCbCr 4:4:4 HDR Gain Map sampling** in their Gain Map implementation.

## Repository structure

| Path | Purpose |
| --- | --- |
| `xdremux/swift-cli/` | Swift CLI entry point. |
| `xdremux/python/` | Python CLI and HEIF I/O helper implementation. |
| `apps/macos/XDRemuxApp/` | macOS SwiftUI app. |
| `tests/` | Converter tests. |
| `fixtures/` | Small test samples and sample notes. |
| `scripts/` | Local build, run, and validation scripts. |
| `experiments/` | Experimental code. |

## Known limitations

- HDR Gain Map and HDR metadata may be lost after editing and saving a converted photo again in OPPO Gallery.

This tool is for technical research only. Back up your original files before conversion. The author assumes no legal responsibility for data loss.

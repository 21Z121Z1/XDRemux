# Active encoding quality and size audit (2026-07-18)

## Scope and reference hierarchy

This audit covers every payload that XDRemux actively encodes in the current
standard, Apple Portrait, and Apple Photographic Styles paths.

| Product path | Actively encoded payload | Highest-quality reference |
| --- | --- | --- |
| Ordinary UHDR | RGB HEVC 4:4:4 Gain Map | decoded private Gain Map JPEG |
| Ordinary LHDR | monochrome HEVC Gain Map | lossless reconstructed PNG raster |
| Apple Portrait | `src.image` Base converted from JPEG to HEIC | decoded first `src.image` JPEG |
| Apple Portrait | RGB HEVC 4:4:4 Gain Map | decoded second `src.image` JPEG |
| Portrait / Styles | disparity and semantic auxiliary images | pre-ImageIO raw auxiliary buffers |
| Photographic Styles | Main10 Linear Thumbnail | lossless generated PNG |
| Photographic Styles | repeated neutral Delta Map tile | lossless constant-value PNG |

Ordinary UHDR/LHDR Base HEVC payloads are remuxed byte-for-byte and are not an
encoding-quality variable. Portrait scaffold Base and Gain payloads are also
restored from the first assembly byte-for-byte; their temporary scaffold
encodes affect time but not final quality. PNG bridges and binary Styles
metadata are lossless.

Visible images were compared as full-resolution RGBA using PSNR, MAE, and
SSIMULACRA2. Gain Maps were compared as parameter rasters using code-value MAE,
P99, and PSNR. Masks use code error and threshold IoU; disparity uses float16
absolute error. Payload size is the affected HEIF item data, not an unrelated
whole-file delta.

## Results

### Portrait `src.image` Base (three real samples)

| ImageIO quality | Total Base payload | Mean SSIMULACRA2 | Mean PSNR |
| ---: | ---: | ---: | ---: |
| 0.80 | 2,866,388 B | 85.552 | 45.497 dB |
| 0.85 | 2,866,388 B | 85.552 | 45.497 dB |
| **0.90** | **5,027,086 B** | **89.733** | **49.412 dB** |
| 0.95 | 7,969,704 B | 91.520 | 53.312 dB |
| 0.99 | 11,341,289 B | 92.308 | 57.316 dB |

`0.80` and `0.85` select the same VideoToolbox tier. `0.90` gains 4.18
SSIMULACRA2 points over that tier; moving to `0.95` costs another 58.5% for
1.79 points. `1.0` is not the highest-quality representation in practice: it
switches to a much larger RExt 4:4:4 Base and was dominated by lower settings
on the medium sample. The selected Base default is `0.90`.

### Gain Map HEVC (nine real UHDR, LHDR, and portrait samples)

| Quality | Total payload | Mean MAE | Mean P99 |
| ---: | ---: | ---: | ---: |
| 0.80 | 2,852,886 B | 0.6751 | 4.44 |
| **0.90** | **5,309,913 B** | **0.4263** | **3.22** |
| 0.95 | 9,308,468 B | 0.2614 | 2.67 |
| 1.00 | 16,968,024 B | 0.1477 | 2.00 |

The selected Gain Map default remains `0.90`. It reduces mean error by 36.9%
and P99 by 1.22 code values versus `0.80`; `0.95` then costs 75.3% more payload
for a smaller absolute improvement. LHDR reaches exact lossless raster
round-trip at `1.0`, but at 2.4 times its `0.90` payload across the three
samples.

The ImageIO path used by `--oppo-compatible` and direct-encoder fallback was
also scanned at `0.80`, `0.90`, `0.95`, and `1.0` on three UHDR and three LHDR
files. All four requested values produced identical Gain payload sizes and
decoded rasters for every sample. The request quality is therefore not an
effective ImageIO Gain Map control; this path retains ImageIO's established
default rather than exposing a fake tuning knob.

The audit found and fixed a separate dominant defect: when height was not a
tile multiple (for example 1532 padded to 1536), Quartz placed padding above
the image while the HEIF grid cropped from the top. This shifted the decoded
Gain Map by four rows. On the high-detail portrait sample, `0.90` MAE changed
from 8.881 before the fix to 0.850 after it (P99 60 to 4).

### Gain Map tile size (six ordinary samples plus three portraits)

| Tile size | Ordinary payload total | Ordinary encode time total | Result |
| ---: | ---: | ---: | --- |
| 256 | 3,570,646 B | 10.08 s | larger and slower |
| **512** | **3,548,649 B** | **3.77 s** | compatible with every path |
| 1024 | 3,540,467 B | 3.00 s | ordinary ImageIO pass; portrait graph mismatch |

Changing 512 to 1024 saved only 0.23% payload across ordinary samples. Apple
Portrait's ImageIO scaffold emits a 512 grid, so both 256 and 1024 fail the
intentional first-assembly/scaffold graph-equivalence check. The production
policy therefore retains 512 tiles and automatically derives rows, columns,
and edge padding from each image. An environment-only override remains for
future compatibility/device experiments; it is not a product default.

### Photographic Styles (three generated Linear Thumbnails)

| Linear quality | Total payload | Mean SSIMULACRA2 |
| ---: | ---: | ---: |
| 0.80 | 591,276 B | 63.257 |
| **0.85** | **719,863 B** | **63.548** |
| 0.90 | 1,093,039 B | 63.778 |
| 0.95 (old) | 1,573,432 B | 63.842 |
| 1.00 | 1,970,643 B | 63.856 |

`0.85` reduces the three Linear Thumbnail payloads by 54.3% relative to the
old `0.95` setting while losing 0.294 mean SSIMULACRA2. Above `0.85`, the next
quality tiers consume much more data for small measured gains. The selected
Linear Thumbnail default is `0.85`.

The neutral Delta Map is one constant 512 tile repeated 30 times. Every tested
quality remained spatially constant. `0.30` reduced its IDR sample from 202 B
to 151 B versus `1.0`, saving about 1.5 KB in a 30-tile grid without losing any
structure. The selected neutral Delta Map default is `0.30`.

### ImageIO disparity and semantic auxiliaries

Setting primary-image lossy quality to `0.5`, `0.8`, or `1.0` produced the
same auxiliary payload sizes and decoded rasters. ImageIO does not expose the
semantic HEVC quality through that option, so the ineffective override was
removed.

| Auxiliary | Encoded payload | MAE | P99 | Max | IoU at 128 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Disparity float16 | 14,705 B | 0.000404 | 0.003540 | 0.014160 | n/a |
| Portrait matte | 39,015 B | 0.0650 | 2 | 12 | 0.999297 |
| Skin matte | 28,830 B | 0.0484 | 1 | 11 | 0.998671 |
| Hair matte | 1,075 B | 0.000320 | 0 | 7 | 1.000000 |
| Teeth matte | 865 B | 0 | 0 | 0 | 1.000000 |
| Glasses matte | 865 B | 0 | 0 | 0 | 1.000000 |
| Sky matte | 1,940 B | 0.001410 | 0 | 7 | 1.000000 |

These seven payloads total 87,295 B on the combined sample. Their current
ImageIO encoding is already small and preserves mask decisions, so replacing
it with a private manual encoder is not justified by the measured Pareto data.

## Selected production policy and limitations

- Portrait Base: `0.90`.
- UHDR/LHDR/portrait Gain Map: `0.90`, 512 tiles.
- Styles Linear Thumbnail: `0.85`.
- Styles neutral Delta Map: `0.30`.
- Semantic/disparity auxiliaries: ImageIO-managed encoding, no fake quality knob.
- `XDREMUX_ENCODING_AUDIT_DIR` can persist the pre-ImageIO auxiliary buffers;
  quality and tile environment overrides exist only for repeatable experiments.

The evidence is macOS VideoToolbox/ImageIO, parser, and decoded-raster evidence.
It does not establish OPPO Gallery or Apple Photos behavior on a physical
device; those remain separate device gates.

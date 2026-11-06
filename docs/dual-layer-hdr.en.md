# Dual-layer HDR conversion

English | [简体中文](dual-layer-hdr.md)

This page states what XDRemux converts today for dual-layer HDR stills, which source facts it keeps, and which work is not implemented. It separates verified behavior from planned behavior. [CLI reference](cli.en.md) documents the commands.

## Supported sources

| Source | Detection | Notes |
| --- | --- | --- |
| ProXDR HEIC from OPPO, OnePlus, or realme | The vendor extension block and its manifest | Local HDR and Ultra HDR families |
| Standalone ISO 21496-1 dual-layer HEIF | A canonical `tmap` graph and no vendor extension block | Includes an earlier XDRemux HEIF output |
| Two-image JPEG | The `MPF` index and the ISO APP2 carriers | Validation only, not conversion |

A Motion Photo takes the Live Photo path. A plain single-image JPEG is not a dual-layer source.

## Supported outputs

| Output | Container | Derived-image declaration |
| --- | --- | --- |
| `compatible` | HEIF | BT.2020 PQ with the existing consumer layout |
| `bt2020-pq` | HEIF | BT.2020 PQ |
| `bt2020-hlg` | HEIF | BT.2020 HLG |
| `.jpg` or `.jpeg` destination | Two-image JPEG | Base colorimetry plus the ISO 21496-1 gain-map parameters |

`batch` writes HEIF output only.

## Source facts the conversion keeps

- The compressed SDR base keeps its bytes on both HEIF paths. A JPEG destination decodes it and re-encodes it.
- The Gain Map keeps its geometry and declared operation, but its pixels are decoded, quantized to eight bits per component, and re-encoded. The resulting samples are not bit-exact, particularly for sources above eight bits.
- The gain-map operation metadata is preserved. A standalone ISO source keeps its exact tone-map payload bytes. A ProXDR source keeps the operation its vendor float view declares.
- `mdcv`, `clli`, and registered `it35` carriers are copied only when the source associates them with the derived HDR image. Two carriers that declare the same value must agree. An absent value stays absent. Optional registered matrix/3D LUT extension bytes are decoded with bounds checks and retained exactly.
- A JPEG destination keeps the base ICC profile and the Exif payload.

The conversion re-reads the published bytes before publication. It checks the structure, the carriers (including actual opt-in APP8 declaration offsets and enhanced payload), the declared profile, and both image decodes.

## Mapping and luminance executors

Two pure executors are available to code that already holds typed inputs. Neither one reads a file.

`AdditionalMapping` applies a three-by-three matrix and then an optional three-dimensional LUT. The matrix runs first in `f64` and keeps negative intermediate values. Each component is then normalized by the LUT domain. Nodes are stored with the first component fastest, and the eight nodes of the containing cell are weighted by their fractional parts. A value outside the domain is rejected by default, or clamped when the caller selects that policy. The fixed vectors cover neutral white, a non-neutral color, the red-fastest axis order, an interior sample of a three-node edge, and a non-unit domain.

`AbsoluteDisplayTarget` gives relative linear `1.0` a luminance through a caller-declared reference white. A component above the peak or below the black level is clamped and counted. `relative_linear_to_pq` encodes the calibrated luminance as PQ. Fixed vectors cover 100, 203, 1,000, and 10,000 cd/m².

Neither executor changes a conversion on its own. The registered file mapping extension is decoded and preserved, but cannot be passed directly to `AdditionalMapping`: its scalar LUT payload does not declare the vector-channel and domain semantics required by that executor. A display target is not read from a device.

## Verification

Run the focused checks from the repository root:

```bash
cargo test -p xdremux-runtime --test hdr_metadata_wiring
cargo test -p xdremux-runtime --test jpeg_export
cargo test -p xdremux-runtime --test jpeg_validation
cargo test -p xdremux-runtime --test jpeg_registered
cargo test -p xdremux-runtime --test iso_heif_source
cargo test -p xdremux-cli --test jpeg_output
cargo test -p xdremux-cli --test iso_heif_source
cargo test -p xdremux-codec --test dual_jpeg
```

`xdremux validate` reports the published file:

```bash
xdremux validate output.heic --hdr-profile bt2020-hlg --semantic --decode-hdr --json
xdremux validate output.jpg --semantic --json
```

The independent ISO/JPEG gain-map check runs when the external tool is installed. It requires that the independent reader reports the same parameters and decodes to identical linear output. The check skips with a message when the tool is absent.

Measured limits for the current encoding configuration:

| Comparison | Mean | Maximum |
| --- | --- | --- |
| JPEG base re-encode drift from the source base | below 2 of 255 | below 24 of 255 |
| JPEG export against the HEIF export of the same source | 0.19 percent relative | 1.3 percent at the 99th percentile |
| Profile change through the ISO graph path | 0.000002 absolute | 0.003 absolute |

These values describe one fixed encoding configuration. They are not a general quality guarantee.

## Limits and open items

- The reconstruction output is relative linear BT.2020. The absolute-luminance executor needs a caller-declared reference white; no conversion supplies one.
- PQ and HLG tags declare an encoding. They do not prove display adaptation, tone mapping, or device presentation.
- Dynamic metadata is parsed into PQ-coded content statistics, target-display parameters, optional base-curve and spline fields, and saturation gains. Exact source bytes are retained. Parameter decoding is not display adaptation: the complete target-dependent tone-mapping algorithm is not implemented.
- A JPEG destination requires identity orientation and a normal Exif orientation.
- A JPEG destination requires the gain operation to run in the base application space. A source that declares the alternate space is refused.
- A JPEG destination cannot carry an explicit BT.2020 profile, the OPPO camera tail, or Apple editing resources.
- The registered JPEG APP8 pair is written only on request (`--jpeg-registered-metadata`) and only when the source supplies enhanced registered metadata. A source without it is refused, rather than publishing a base declaration with no enhanced record. The reduced-view offset follows a documented rule that an independent producer sample has not confirmed yet.
- The Gain Map encoder currently accepts eight-bit rasters; higher-bit-depth sources are quantized before encoding. Geometry and metadata preservation do not guarantee sample-exact preservation.
- Registered matrix/3D LUT sections can be read from and round-tripped through HEIF `it35` and JPEG APP8. Precision, finite values, sizes, and cube dimensions are validated. Their rendering semantics are not established, so the conversion preserves them without applying them to pixels; no implicit RGB-vector interpretation or LUT domain is invented.
- No device evidence is recorded here. Gallery recognition, Photos editing, and display behavior are outside these checks.

## Code ownership

| Concern | Crate |
| --- | --- |
| Container facts, carrier extraction, HEIF assembly | `crates/xdremux-heif` |
| JPEG index, `MPF`, two-image assembly | `crates/xdremux-format` |
| Source analysis, conversion, published-file checks | `crates/xdremux-runtime` |
| Request, container, and profile policy | `crates/xdremux-engine` |
| Decoding, encoding, and Gain Map tiles | `crates/xdremux-codec` |

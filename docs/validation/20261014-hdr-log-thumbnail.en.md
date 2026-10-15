# HDR Log Linear Thumbnail validation

English | [简体中文](20261014-hdr-log-thumbnail.md)

The tested change replaces copied SDR codes in the Styles Linear Thumbnail with
the Apple Log curve applied to reconstructed linear Display P3 HDR. Exposure
normalization uses Styles plist key `4`. Existing key1 synthesis and auxiliary
color signaling remain in use.

## Encoded-resource regression

The native regression runs the Rust Styles transaction on two versioned ProXDR
fixtures. It decodes the VideoToolbox Main10 thumbnail with libheif, compares it
with the uncompressed HDR Log target, and checks the unchanged primary HEVC
payload and configuration. Error units are 8-bit RGB code values.

| Fixture | Decoded thumbnail | HDR Log target MAE | SDR image MAE |
| --- | --- | ---: | ---: |
| `oppo/find-x9-ultra/uhdr-portrait-01.heic` | 770 × 1024 | 1.5852 / 255 | 32.9170 / 255 |
| `oppo/find-x7-ultra/lhdr-v2-01.heic` | 1024 × 878 | 0.9890 / 255 | 42.3492 / 255 |

The UHDR fixture exercises RGB gain; the LHDR fixture exercises monochrome gain.
The acceptance bound is HDR Log MAE below 3 / 255. The comparison also rejects
a thumbnail that still resembles the copied SDR raster. Unit tests cover the
log toe, values above SDR white, exposure normalization, channel order, geometry,
and rejection of invalid input.

As a negative control, restoring the original SDR-copy statement makes the same
real-media test fail on the UHDR fixture with HDR Log MAE 32.0010 / 255. The
corrected statement was restored before the final build and completion gate.

The codec currently takes RGB8 input and writes Main10 4:2:0. These results do
not imply lossless 10-bit RGB reconstruction.

## Matched native-render diagnostic

Two baseline files were generated with only the original SDR-copy statement
restored. The corrected files use the new statement. The fixture, adapter,
codec settings, and key1 policy are the same. Among item payloads, only the
thumbnail changes: all other 103 UHDR and 115 LHDR item payloads, including the
Styles property list, are byte-identical. The encoder supplies the thumbnail's
matching HEVC configuration.

The diagnostic rendered disabled, neutral, Tone −1, Tone +1, Color −1, Color +1,
Tone −1 / Color +1, and Tone +1 / Color +1. It used Standard cast, intensity 1,
and maximum dimension 1024. There were 32 successful renders in total.

On macOS 27.0.1 build 26A434, the Neutrino helper produced identical pixels for
all settings within each file. It also produced identical pixels between the
baseline and corrected files. The helper reports the requested settings and
successful PNG output, but this run does not establish an active Styles
adjustment. The current product emits version 15 Styles metadata; the earlier
responsive research carrier used version 16. This comparison therefore proves
no visible Tone or Color improvement on the current product carrier.

The helper SHA-256 was
`476a36a38a88f6860c8dbb6c21133c1c40e1d0dc565a4c257c03e632be51afd7`.
Keep this result distinct from the earlier RAW-reference error and the separate
complete-key1-solver experiment. Neither is an acceptance metric for this PR.

## Reproduction and limits

Run the native resource regression and existing Styles consumer checks with:

```bash
bash scripts/apple/check_rust_style_consumer.sh
```

The script supplies the built Apple adapter to the new Rust test. The test is
skipped during portable tests when the adapter is absent. The macOS consumer
gate runs it with the adapter present.

The supported result is a corrected encoded HDR reference with unchanged
primary HEVC and Styles metadata. This does not establish RAW recovery,
camera-authentic producer state, lower total conversion time, or Photos
import/edit/save/reopen behavior. Photos and device HDR display remain separate
validation work.

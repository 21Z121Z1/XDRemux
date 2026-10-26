# Device and file compatibility

English | [简体中文](supported-devices.md)

XDRemux selects a conversion path from the input file's structure and metadata. A phone model is not an input allow-list. One successful file does not establish support for every firmware version, camera mode, or file from that model.

## Coverage backed by repository fixtures

The versioned corpus provides concrete inputs for repeatable checks:

| Asset | Included sources | What the fixtures establish |
| --- | --- | --- |
| ProXDR | OPPO Find X6 Pro LHDR v1; Find X7 Ultra LHDR v2, including XPAN; Find X9 Ultra UHDR, including high-resolution, Master, and Portrait captures. | The format and product-policy cases exercised by the associated tests. |
| Motion Photo | Samsung, Xiaomi, OPPO, and vivo JPEG and HEIC/HEIF inputs. | The parser, conversion, and validation cases exercised by the associated tests. |

The [fixture guide](../fixtures/README.en.md) describes the corpus. [SHA256SUMS](../fixtures/SHA256SUMS) identifies the exact files. These checks do not establish every camera mode or acceptance by the original Gallery or Apple Photos.

## Recorded ProXDR capture models

The following catalogue preserves the models reported in the project's device documentation. The repository does not provide a separate fixture or verification record for every entry. Treat this table as capture-format context, not a verified support matrix.

| Brand or series | Models |
| --- | --- |
| OnePlus | OnePlus Ace2 Pro, OnePlus 12, OnePlus Ace3, OnePlus Ace 3V, OnePlus Ace 3 Pro, OnePlus 13, OnePlus Ace 5 series, OnePlus 13T, OnePlus Ace 6, OnePlus Ace 6T, OnePlus Turbo 6, OnePlus 15, OnePlus 15T, OnePlus Ace 5 Supreme Edition |
| OPPO K series | K12, K12x, K13 Turbo series, K15 Pro series |
| OPPO Find series | Find X6, Find X6 Pro, Find N3, Find N3 Flip, Find X7, Find X7 Ultra, Find X8 series, Find N5, Find X8s, Find X9 series, Find N6 |
| OPPO Reno series | Reno10 Pro, Reno10 Pro+, Reno11 Pro, Reno12 series, Reno13 series, Reno14 series, Reno15 series, Reno 16 series |
| realme GT series | realme GT5 series, realme GT5 Pro, realme GT6, realme GT7 Pro, realme GT7 Pro Racing Edition, realme GT7, realme Neo7 Turbo, realme GT8, realme GT8 Pro |
| realme Neo series | realme GT Neo6 SE, realme GT Neo6, realme Neo7, realme Neo7 SE, realme Neo7x, realme Neo8 |
| realme number series | realme 12 Pro, realme 12 Pro+, realme 13 Pro+, realme 13 Pro Supreme Edition, realme 13 Pro, realme 14 Pro+, realme 14 Pro, realme 14, realme 15, realme 15 Pro |

## Gain Map differences

ProXDR files can contain different Gain Map layouts. Earlier project reports describe YCbCr 4:4:4 HDR Gain Maps from OPPO Find X8 Ultra, the Find X9 series, and realme GT8 Pro in Ricoh mode. Other files can use 4:2:0 or monochrome Gain Maps. These reports do not replace inspection of the actual input.

The standard path preserves source Gain Map characteristics where the selected output supports them. `--oppo-compatible` can convert the map to a compatibility representation. The phone name alone does not determine the Gain Map layout.

## Motion Photo requirements

Motion Photo support depends on detectable resources and tested format behavior. The ProXDR model catalogue does not control it.

An input needs a still resource, a motion-video resource that the parser can resolve, and the timing and container information required by the conversion. See the [LibLivePhoto integration contract](liblivephoto-integration.en.md) for exact-time handling and omitted resources.

## Apple Portrait requirements

Apple Portrait conversion needs compatible portrait resources in the individual source photo. A ProXDR capture model does not imply that every photo contains depth, focus, semantic masks, or restore-original data.

The complete and OPPO source-native profiles have different resource requirements. See the [Apple feature guide](apple-features.en.md) before selecting a profile.

## Report an unsupported file

Keep personal photo content private unless publication is intentional. Redacted container diagnostics can be sufficient to investigate a problem.

1. Keep the original file.
2. Record the device model, OS version, and camera mode.
3. Record the exact XDRemux error.
4. Provide redacted diagnostics or an authorized reproducible sample.

A new compatibility statement requires a reproducible file or test. The model name alone is insufficient.

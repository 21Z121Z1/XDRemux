# Supported Devices

English | [简体中文](supported-devices.md)

This document lists device models that are known to capture ProXDR HEIC.

A listed model is not a guarantee for every firmware version, camera mode, or individual file. XDRemux validates the file structure and metadata that the selected conversion path requires.

## Known ProXDR capture models

| Brand or series | Models |
| --- | --- |
| OnePlus | OnePlus Ace2 Pro, OnePlus 12, OnePlus Ace3, OnePlus Ace 3V, OnePlus Ace 3 Pro, OnePlus 13, OnePlus Ace 5 series, OnePlus 13T, OnePlus Ace 6, OnePlus Ace 6T, OnePlus Turbo 6, OnePlus 15, OnePlus 15T, OnePlus Ace 5 Supreme Edition |
| OPPO K series | K12, K12x, K13 Turbo series, K15 Pro series |
| OPPO Find series | Find X6, Find X6 Pro, Find N3, Find N3 Flip, Find X7, Find X7 Ultra, Find X8 series, Find N5, Find X8s, Find X9 series, Find N6 |
| OPPO Reno series | Reno10 Pro, Reno10 Pro+, Reno11 Pro, Reno12 series, Reno13 series, Reno14 series, Reno15 series, Reno 16 series |
| realme GT series | realme GT5 series, realme GT5 Pro, realme GT6, realme GT7 Pro, realme GT7 Pro Racing Edition, realme GT7, realme Neo7 Turbo, realme GT8, realme GT8 Pro |
| realme Neo series | realme GT Neo6 SE, realme GT Neo6, realme Neo7, realme Neo7 SE, realme Neo7x, realme Neo8 |
| realme number series | realme 12 Pro, realme 12 Pro+, realme 13 Pro+, realme 13 Pro Supreme Edition, realme 13 Pro, realme 14 Pro+, realme 14 Pro, realme 14, realme 15, realme 15 Pro |

This list records known capture support. It is not a code allow-list.

## Gain Map differences

Known files can contain different Gain Map layouts.

OPPO Find X8 Ultra, the Find X9 series, and realme GT8 Pro in Ricoh mode can use YCbCr 4:4:4 HDR Gain Maps in known implementations.

Other files can use 4:2:0 or monochrome Gain Maps.

The standard conversion path preserves source Gain Map characteristics when the selected output path supports them. `--oppo-compatible` can reduce the representation to a compatibility form.

Do not infer the Gain Map layout from the phone name alone.

## Motion Photo support

Motion Photo support is capability-based and fixture-tested. It is not controlled by the ProXDR model table above.

A Motion Photo input must contain a still resource, a motion-video resource that the parser can resolve, and the required timing/container information.

The current public fixture set contains multiple Android Motion Photo layouts. See the [fixture guide](../fixtures/README.en.md).

## Apple Portrait support

Apple Portrait conversion requires compatible portrait resources in the individual source photo.

A supported ProXDR model does not imply that every photo contains depth, focus, semantic, or restore-original data.

## Reporting a new file

If a new device or firmware produces a file that XDRemux cannot convert:

1. Keep the original file.
2. Record the device model, OS version, and camera mode.
3. Include the exact XDRemux error.
4. Include redacted container diagnostics when they are sufficient.
5. Do not publish personal photo content unless you intend to make it public.

A new compatibility claim should have a reproducible file or test. The device model alone is not sufficient evidence.

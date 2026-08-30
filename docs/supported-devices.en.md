# Supported Devices

English | [简体中文](supported-devices.md)

This document lists known ProXDR capture families. It is not a guarantee for every firmware version, camera mode, or individual file.

XDRemux validates the input file, not only the device model. A listed phone can produce photos that do not contain the data required by a selected conversion path.

## Known ProXDR capture families

| Brand or series | Known models or families |
| --- | --- |
| OnePlus | Ace 2 Pro, 12, Ace 3 family, 13 family, Ace 5 family, Ace 6 family, Turbo 6, 15 family |
| OPPO K | K12 family, K13 Turbo family, K15 Pro family |
| OPPO Find | Find X6 family, Find N3 family, Find X7 family, Find X8 family, Find N5, Find X9 family, Find N6 |
| OPPO Reno | Reno10 Pro family through later ProXDR-capable Reno generations documented by project samples and reports |
| realme GT | GT5 family, GT6, GT7 family, GT8 family |
| realme Neo | GT Neo6 family, Neo7 family, Neo8 |
| realme number series | ProXDR-capable 12 through 15 series models documented by project samples and reports |

The table intentionally groups models when the converter contract depends on file structure rather than a marketing model name.

## Gain Map differences

Known files can contain different Gain Map layouts.

Some newer devices and modes can use three-channel 4:4:4 Gain Maps. Other files use 4:2:0 or monochrome Gain Maps.

The standard conversion path preserves the source channel characteristics when the selected output path supports them. `--oppo-compatible` can reduce the representation to a compatibility form.

Do not infer the Gain Map layout from the phone name alone.

## Motion Photo support

Motion Photo support is capability-based and fixture-tested. It is not documented as a phone allow-list.

A Motion Photo input must contain a still resource, a valid motion-video resource, and timing/container data that the parser can resolve.

The current public fixture set contains multiple Android Motion Photo layouts. See the [fixture guide](../fixtures/README.en.md).

## Apple Portrait support

Apple Portrait conversion requires compatible portrait resources in the individual source photo.

A supported ProXDR device does not imply that every photo contains depth, focus, semantic, or restore-original data.

## Reporting a new file

If a file from a new device or firmware cannot be converted:

1. Keep the original file.
2. Record the device model, OS version, and camera mode.
3. Include the exact XDRemux error.
4. Include redacted container diagnostics when they are sufficient.
5. Do not publish personal photo content unless you intend to make it public.

A new compatibility claim should be supported by a reproducible file or test, not only by the model name.

# Palette admission experiments

English | [简体中文](README.md)

These experiments test whether ColorOS Gallery accepts two candidate Palette representations. **The consumer result remains unverified.** The tools do not add a Palette conversion mode to XDRemux.

A successful container inspection proves only that the inspector can read that candidate. It does not prove that Gallery accepts the file, shows Palette controls, or preserves reversible edits.

## Experiment definitions

[`probe.py`](probe.py) preserves the questions from PRs #44 and #45. It uses an isolated parser and does not restore the retired Python product parser.

| Command | Change | Evidence limit |
| --- | --- | --- |
| `context` | Copy source-present `basictone.info`, `basictone.lmtlut.table`, `basictone.vig.table`, and `hdr.transform.data` onto a separate HEIC base. | The base must come from the same source image. The tool does not verify image identity from decoded pixels. |
| `filter-info` | Redirect the manifest's `filter.info` entry to a new 220-byte, little-endian `FilterPhotoInfoV51` payload. | All inserted values are synthetic inputs. They are not measurements of the source image. |

The `context` command excludes filter, watermark, preset, Portrait, and Motion Photo entries. This isolates render-context entries for comparison. It is not a lossless photo conversion.

The public fixture [`coloros16-dualstream-ultrahdr-01.jpg`](../../fixtures/motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg) contains only `hdr.transform.data` from the allowlist. It supplies no evidence that BasicTone tables were present or that Gallery used them.

The `filter-info` command retains the historical `palette-default` and `default` identifiers. Its synthetic values are:

| Field | Candidate value |
| --- | --- |
| Version | 5.1 |
| Intensity | 100 |
| Filter/Palette flags | Flags set by the selected hypothesis |
| Temperature | 6500 |
| Lux | 100 |
| Saturation and tone | 0 |
| Luma values | 0.18 / 0.75 / 0.03 |
| Capture mode | Explicit command argument; `common` in the example below |

## Preservation and parser limits

The filter experiment retains every original byte before the manifest. This includes unmanifested gaps and the replaced filter payload. It adjusts backward offsets, preserves unknown fields, and retains the order of untouched entries.

The parser rejects duplicate entry names, duplicate JSON keys, overlapping extents, invalid integer types, and out-of-bounds footers. Boolean offsets are not valid integers for this format. The parser supports only the observed trailing JSON/NUL/four-byte-tag/little-endian-span layout. Other layouts fail explicitly.

## Run an experiment

Run these commands from the repository root. Install Python 3 and build the Rust CLI before using the inspector command. Use a new output path. The tool refuses to replace existing files or symbolic links.

```sh
python3 research/palette/probe.py filter-info \
  fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic \
  candidate.heic --filter-type palette-default --capture-mode common

target/debug/xdremux inspect candidate.heic --json
python3 -m unittest tests.test_palette_research
```

For a `context` experiment on macOS:

1. Use `sips` to make a separate HEIC base from the source JPEG.
2. Run `probe.py context SOURCE OUTPUT --base-heic BASE` with that JPEG and its HEIC base.
3. Save the JSON report with the corresponding device observations.

Never use the render context of a different image. The repository records the public source fixture identity in [`fixtures/SHA256SUMS`](../../fixtures/SHA256SUMS).

The report uses schema `xdremux-palette-experiment-v1`. It records source, base, and output hashes, retained entries, and the selected hypothesis. It also records `research_only: true` and `consumer_verdict: unverified`.

The tool completes a temporary file in the destination directory, then publishes it through an atomic hard link. Publication refuses to overwrite an existing destination. Filesystems without hard-link support fail; there is no overwrite fallback. See the standard-library contracts for [`os.link`](https://docs.python.org/3/library/os.html#os.link) and [`tempfile.mkstemp`](https://docs.python.org/3/library/tempfile.html#tempfile.mkstemp).

## Requirements before product use

For each candidate, record its exact hash, the source hash, device, ColorOS version, and Gallery version. Check import, control visibility, editing, saving, reopening, and continued reversibility as separate outcomes.

Retain negative results. Do not change unrelated metadata merely to make a control appear. Product use requires a contract that preserves source facts, documented consumer behavior, and a real-fixture regression through the canonical Rust path.

The old one-shot workflows and their obsolete parser dependency are retired. This experiment does not download firmware, upload private media, or declare device validation automatically.

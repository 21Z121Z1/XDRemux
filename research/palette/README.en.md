# Palette admission experiments

English | [简体中文](README.md)

Status: **research; consumer verdict unverified**. These tools do not add a
Palette mode to the Rust product. An inspector accepting the container does
not establish ColorOS Gallery admission or reversible editing.

## Two hypotheses, one tool

`probe.py` reconstructs the useful intent of PRs #44 and #45 without restoring
the deleted Python product parser.

- `context` carries only source-present `basictone.info`,
  `basictone.lmtlut.table`, `basictone.vig.table`, and `hdr.transform.data` onto
  a HEIC base decoded from the same source. It excludes filter, watermark,
  preset, Portrait, and Motion Photo entries. This is an ablation, not a
  lossless photo conversion. The original public ColorOS 16 fixture, now at
  `fixtures/motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg`, contains
  only `hdr.transform.data` from this allowlist. Do not describe it as evidence
  that BasicTone tables were present or consumed.
- `filter-info` changes only the manifest's `filter.info` reference to a new
  220-byte little-endian `FilterPhotoInfoV51` hypothesis. Both historical
  `palette-default` and `default` identifiers remain reproducible. Version
  5.1, intensity 100, filter/Palette flags, temperature 6500, lux 100,
  saturation/tone zero, luma values 0.18/0.75/0.03, and the explicit capture
  mode are **synthetic experimental inputs**, not measurements of the photo.

The filter experiment retains every original byte before the manifest,
including unmanifested gaps and any replaced filter payload. It adjusts
backwards offsets and preserves unknown fields and untouched entry ordering.
It never repacks unrelated data. The parser rejects duplicate names/JSON
keys, overlapping extents, invalid integer types, and out-of-bounds footers.
Only the observed trailing JSON/NUL/four-byte-tag/little-endian-span layout is
supported; unsupported layouts fail explicitly.

## Reproduction

Run from the repository root. Use a new output path; existing files and
symlinks are never replaced.

```sh
python3 research/palette/probe.py filter-info \
  fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic \
  candidate.heic --filter-type palette-default --capture-mode common

target/debug/xdremux inspect candidate.heic --json
python3 -m unittest tests.test_palette_research
```

On macOS, generate a separate HEIC base from the same JPEG with `sips`, then
run `probe.py context SOURCE OUTPUT --base-heic BASE`. The source fixture
identity is recorded in `fixtures/SHA256SUMS`. Never use a different image's
render context. The source and base hashes, output hash, retained entries,
and hypothesis are printed as JSON. Save that report with device observations.

Publication uses a completed same-directory temporary file and an atomic
no-clobber hard link. A filesystem without hard-link support fails explicitly;
there is no overwrite fallback. See the Python standard-library contracts for
[`os.link`](https://docs.python.org/3/library/os.html#os.link) and
[`tempfile.mkstemp`](https://docs.python.org/3/library/tempfile.html#tempfile.mkstemp).

## Promotion gate

Record the exact candidate hash, source hash, device, ColorOS version, and
Gallery version. Check import, control visibility, edit, save, reopen, and
continued reversibility separately for each candidate. A negative result is
valuable evidence; preserve it rather than changing unrelated metadata until
a control appears. Promote only a source-truthful contract supported by that
consumer evidence and a real-fixture regression on the canonical Rust path.

The old one-shot workflows and their obsolete parser dependency are retired.
No firmware extraction, private media upload, or automatic device-success
claim is part of this experiment.

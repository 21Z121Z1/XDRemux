# Styles fidelity experiments

[简体中文](README.zh-CN.md)

This research prepares a blind comparison between original Apple Styles resources
and resources reconstructed from the same photo's SDR/HDR information. It does
not change product generation, calibration constants, or supported input types.

## Prepare the corpus

```sh
python3 -m research.styles_fidelity.corpus \
  --output /absolute/path/new-corpus /absolute/path/photos.zip
```

The catalog records archive and member identities, exact file duplicates, coded
primary fingerprints, associated Styles resources, and typed SmartStyle state.
Original HEIF files use hash-based local names. Archive paths are never extracted.
The script does not execute code or extract system binaries from an archive.

A Standard candidate needs Apple EXIF, the established neutral SmartStyle fields,
the relevant resources, and no non-Standard Texture preset. This is a calibration
selection rule. Neither metadata nor archive membership certifies capture origin.
Files with missing state remain unknown. A Texture preset alone is insufficient.

The reference catalog reports baseline values outside the current product's
`[4, 10.4]` calibration range. That diagnostic does not prove render error or
authorize changing the range. Native rendering must determine the effect.

## Isolate producer inputs

```sh
python3 -m research.styles_fidelity.isolate \
  --input /absolute/path/original.heic --output /absolute/path/new-case
```

The allow-list projection retains the compressed primary, one associated Apple
HDR Gain Map, their coded dependencies, verified pixel properties, a matching
tone-map item when present, admitted source HDR format brands, item visibility,
and numeric HDR descriptors. It creates a new item
graph and new media storage. Original Styles, Linear Thumbnail, Delta Map,
semantic resources, SmartStyle MakerNotes, unrelated XMP and unused media bytes
do not enter the producer file. EXIF is rebuilt from scratch with the literal
Apple make verified in the source and only numeric HDR MakerNote tags 33/48, preserving their rational
values. All other EXIF fields and MakerNote state are omitted. Removing the HDR
fields can preserve SDR pixels while disabling Apple's HDR expansion, so the
native check is essential. Pixel payloads, color and geometry are checked again.
Unsupported properties or ambiguous relationships fail.

Review scene groups before fitting. Supply a JSON object with
`scene_groups_reviewed: true` and a `samples` dictionary keyed by candidate SHA.
Each entry needs `scene_group` and `split` (`calibration` or `holdout`).

```sh
python3 -m research.styles_fidelity.split \
  --corpus /absolute/path/new-corpus/corpus.json \
  --groups /absolute/path/reviewed-groups.json --output /absolute/path/new-split
```

One scene and one coded primary cannot cross splits. The producer manifest has
isolated inputs; the separate oracle manifest has references and labels. The
split applies only to this experiment and does not establish that earlier
research never used a photo. Native equivalence remains unverified until measured.

## Verify native equivalence

Build the existing Apple adapter on its supported arm64/macOS host, then run:

```sh
python3 -m research.styles_fidelity.native \
  --adapter /absolute/path/xdremux-apple-adapter \
  --input /absolute/path/original.heic --output /absolute/path/new-native-result
```

The check compares complete, oriented RGBA32F linear Display P3 SDR and HDR
readback from the original and isolated file. It requires unchanged source bytes,
headroom, finite pixels and maximum absolute error at most `1e-6`. The reference
must actually expand HDR. Temporary pixel buffers are deleted after comparison.
Only numeric facts, dimensions and hashes enter the report. Existing outputs are
never reused. Clean-aperture geometry is currently rejected.

Use `--ablate-hdr-exif` to also compare a stricter projection without numeric HDR
EXIF. That comparison records whether EXIF is needed by the tested file's HDR
representation; it does not waive a failure in the default projection.

The existing `research.yml` tone job runs this check on its pinned public Apple
samples. Uploads contain only the report. Supplied private photos stay outside Git
and public artifacts. Portable parsing or decoding does not prove Apple HDR
equivalence, reconstructed slider fidelity, or Photos editing and save/reopen.

## Next experiments

1. Admit each private corpus case only after native SDR/HDR equivalence passes.
2. Call the shared Rust resource producer on isolated inputs. Keep original
   coefficients, curves, mattes and statistics accessible only to the evaluator.
3. Compare native nonzero Tone/Color and remove-style results before changing
   exposure, GTC or local-fit calibration. Freeze scene splits before fitting.
4. Apply the established final-file Texture replay to full HDR product outputs,
   recording decoded masks, color domain, scale and per-effect response.
5. Keep same-frame face-boost curve capture and RAW validation separate from
   metadata synthesis. Preserve uncertainty where the inverse is non-unique.

Run portable boundary regressions with
`python3 -m unittest tests.test_style_fidelity -v`.

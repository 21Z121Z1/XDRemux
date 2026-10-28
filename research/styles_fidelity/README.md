# Styles fidelity experiments

[简体中文](README.zh-CN.md)

This research compares original Apple Styles resources with resources rebuilt
from the same photo's isolated SDR/HDR information. It also replays the native
Texture effects with masks and person data decoded from final HDR product files.
Product defaults and supported input types are unchanged.

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
tone-map item when present, its ordered alternate-image group, admitted source
HDR format brands, item visibility,
numeric HDR descriptors, and source `idat`/`mdat` construction methods. Compact
item IDs use the 16-bit HEIF layouts. It creates a new item
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

## Regenerate and jointly calibrate resources

The `styles-research` Cargo feature exposes a research example that calls the
shared Rust Styles producer on an isolated Apple HDR base. It does not disguise
the source as ProXDR or expand product CLI admission. The engine accepts explicit
baseline and gain scales plus a baseline maximum. Defaults preserve the existing
product calibration. Each trial rebuilds the thumbnail, GTC, coefficients, light
maps, statistics and metadata together; original resources are evaluator-only.
For the research Apple HDR input, semantic resources are merged into the
authoritative source graph. The source primary, Gain Map, transforms and color
properties remain intact. A Gain Map identified by the ordered tmap graph may
link to the primary alone or to both primary and tmap; semantic mattes still
require both targets. Unrelated or duplicate Gain Map targets remain errors.
The merge preserves the exact source HDR MakerNote rationals 33/48 alongside
the scaffold's newly generated editing state. Re-encoding those source facts
through ImageIO's number representation must not change the HDR base. Each
trial checks their exact values before evaluating edit responses.

```sh
cargo build --locked -p xdremux-runtime --features styles-research --example styles_fidelity
python3 -m research.styles_fidelity.blind \
  --adapter /absolute/path/xdremux-apple-adapter \
  --generator /absolute/path/target/debug/examples/styles_fidelity \
  --research-root /absolute/path/pinned-consumer-research \
  --calibration /absolute/path/calibration-a.heic \
  --calibration /absolute/path/calibration-b.heic \
  --holdout /absolute/path/holdout.heic --output /absolute/path/new-fit
```

Every case must pass full native SDR/HDR input equivalence. Scene assignments are
fixed before response evaluation. A default trial and a 3-by-3 exposure/gain grid
are ranked only by calibration-set native forward Tone/Color RMSE. The holdout is
evaluated after selection. Color runs only when the native filter exposes its
control. Numeric comparisons include shadow, midtone and highlight regions.
A failed default generation or forward render blocks selection.
Native remove-style remains a separate required validation on the source and
rebuilt files. Its failure does not discard independently completed forward
measurements. Schema v2 distinguishes `forward_calibration_completed` from
`native_reverse_validation_passed`; overall `passed` and the process exit remain
failed when reversal fails. A forward-only candidate cannot establish complete
reconstruction fidelity or be promoted to product defaults.
The renderer uses the pinned `native-stage-mirror` path: native Apple Log decode,
stored thumbnail gain and original-range limits precede the filter input.
The probe's stored-code control path is excluded from calibration.
The reverse probe compiles the current product's native primitive unchanged.
Its owned caller converts a native assertion abort into a reported exception for
diagnosis; the assertion still fails native reverse validation and overall
acceptance. The probe does not override the resource-cache implementation.

The CI controls are the three published semantic-Standard `IMG_2600/2604/2608`
files. Their Studio Texture metadata has no people, so it cannot activate a
person effect. They are public landscape controls with prior research exposure;
they do not replace the stricter private Standard corpus or establish portrait
or night-scene generalization. The existing private corpus eligibility and
scene split are unchanged. Private native fitting requires a private Mac host;
private photos are never sent to public Actions or artifacts.

## Replay final HDR Texture resources

```sh
python3 -m research.styles_fidelity.texture \
  --adapter /absolute/path/xdremux-apple-adapter \
  --research-root /absolute/path/pinned-texture-research \
  --input /absolute/path/current-product.heic --output /absolute/path/new-replay
```

The pinned Texture primitives come from the research revision named in
`texture.py`. The source must have native people and actually exercise pixels
above SDR white. Masks are projected from final encoded resources and decoded
by ImageIO. Native final-file person geometry and statistics are reused; no
fresh Vision observations or pre-encoding masks enter the replay. Dimensions
must match the product's 1024-edge working domain without a mask resize fallback.

Mattify, UnderEyeBrighten and SkinSmoothStandalone are rendered independently
per person in two controlled domains: the product's SDR-derived linear P3
analysis image and area-reduced native linear P3 HDR. The latter bypasses the
8-bit SDR upload and retains values above one. Each effect must read back the
exact submitted HDR pixel hash; all effects in a domain must receive identical
working pixels. Reports retain these input facts, output maxima, GPU status,
finite-pixel checks, output changes and stored-versus-recomputed statistic
differences. Each domain renders once with stored statistics and once after
clearing statistics while retaining the exact final-file person geometry.
The latter recalculates them from decoded final masks. Their pixel difference
measures sensitivity to statistics computed before resource encoding.
Differences across the two domains measure domain sensitivity;
they are not an error against a Photos reference render. This is not full-size
effect composition, multi-person blending or a device save/reopen test.

The existing research tone job converts `uhdr-portrait-02.heic` with the current
product CLI, executes both experiments and exports only numeric reports.
This public fixture depicts shop mannequins, not human skin. Its native detected
person context exercises resource replay but does not establish human portrait
fidelity. `uhdr-portrait-01.heic` depicts a toy and has no native face context.
Images,
landmarks, input resources and render buffers remain in owned temporary storage.

## Remaining experiments

1. Admit each private corpus case only after native SDR/HDR equivalence passes.
2. Promote calibration only after broader independent scene coverage and
   native edit responses support it. Retain product defaults otherwise.
3. Compare Texture replay with the complete Photos composition and real-device
   editing, including multi-person interactions and full-resolution output.
4. Keep same-frame face-boost curve capture and RAW validation separate from
   metadata synthesis. Preserve uncertainty where the inverse is non-unique.

Run portable boundary regressions with
`python3 -m unittest tests.test_style_fidelity -v`.

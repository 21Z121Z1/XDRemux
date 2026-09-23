# Style reconstruction research

English | [简体中文](README.md)

This is a maintained research surface, not a Rust product provider. Predicted
`key1`, GTC, light maps and capture scalars are candidates, not recovered capture
facts. No model, offline metric, parser pass or threshold here establishes native
consumer equivalence or Apple Photos reversible editing.

## Owners and inputs

`training.py` owns single-primary and masked-modality model experiments. It uses
the canonical paired/self-pair research primitives in
`xdremux_py/apple_reverse_key1_training.py`; there is no second product converter.
`inference.py` reads inputs and builds finite candidate payloads. `public_pretraining.py`
uses public content with analytic synthetic affine labels, not Apple producer labels.

Integer primary RGB is interpreted in `[0, 255]`; floating RGB must be in `[0, 1]`.
The interpretation depends on dtype, not image extrema: a byte-valued dark image
with maximum 1 must not become full-white. Inputs are oriented `3 × 256 × 256` RGB.
The model's nine primary features are RGB, luma, Cb, Cr, log-luma and two gradients.

Optional linear RGB sidecars contain oriented scene-linear values in `[0, 1]`.
Decoded gain-map sidecars contain positive linear exposure ratios, with 1 neutral.
Both have explicit availability masks and SHA-256 identities. A DNG embedded
preview is RGB, not decoded RAW. A marker string is not a decoded gain map. The
legacy model feature names `has_raw` and `has_gain_map` mean supplied decoded
modalities; suffixes and marker searches never set them. The original private
checkpoint did not learn positive examples for all these modalities.

Metadata has a separate missingness mask. Boolean EXIF values do not become
numeric capture measurements. Missing scalar targets do not contribute a loss
or optimize task uncertainty; their evaluation metric is `null`, not zero.

## Dataset and evaluation contract

The universal dataset v2 pins the source manifest, extracted label archive and
every cached sample. It verifies identities on the bytes it decodes, validates
array shapes, masks and finite values, and rejects repeated source identities,
invalid label indices and capture-session leakage. Preparation checks native
source identities before and after metadata extraction. Reprepare legacy v1
manifests from verified inputs rather than silently certifying missing identities.

GTC supervision preserves the complete 516-byte mixed-layout Tag3 resource as
bytes. Treating it as a flat Float16 array is invalid. A 516-byte *prediction*
still does not prove that structural fields form a valid native GTC resource.

The three model variants (`base`, `multiscale_large`, `multimodal_large`) remain
research alternatives. Warm starts must include every learned parameter and keep
target-corpus statistics; newly added modality channels start at zero. `--resume`
is manifest-locked **weights-only continuation**, not optimizer/RNG-exact resumption.
Every run requires a new output directory, including continuation runs. Existing
checkpoints and output symlinks are never reused. Non-finite predictions, losses,
gradients or selection metrics stop the run; a failed selection cannot inherit an
older `best.pt`. Checkpoint writes are staged atomically inside the new run directory.
The manifest identity is pinned before fitting and checked before publishing evidence.
Calibration chooses the checkpoint; heldout evaluation follows that choice.
`consumerProxyRMSE8` uses squared pixel residuals, not squared per-image MAE.
The encoded-RGB quadratic proxy is a training regularizer, not Apple's renderer.

Public pretraining requires three or more independent source images. It freezes
source-disjoint train/calibration/heldout partitions before fitting and evaluates
heldout only after freezing selected weights. Earlier experiments used a set
called “heldout” for checkpoint selection: those measurements are selection-set
metrics and must not be reused as independent heldout evidence.

The collector records publisher-provided license metadata and attribution. Its
allowlist is not an independent legal determination. Revision-pinned scikit-image
sources are verified by Git blob identity before decoding. Commons original-file
SHA-1 and resized-download SHA-256 are different identities and are not compared
as though the bytes were identical. Downloaded tensors are separately hashed.
Existing corpus receipts and tensors are not overwritten.

## Commands

Install research dependencies with `python -m pip install '.[training]'`.
Private native media, caches, checkpoints and raw per-image reports are not
published with the repository.

```bash
python -m research.oppo_styles.train prepare --native-manifest /data/native/manifest.json --output /data/universal-v2
python -m research.oppo_styles.train train --manifest /data/universal-v2/manifest.json --output /data/run
python -m research.oppo_styles.pretrain collect --curated-only --output /data/public.json --image-directory /data/public-images
python -m research.oppo_styles.pretrain train --manifest /data/public.json --output /data/synthetic-run
python -m research.oppo_styles.predict --input /data/photo.heic --checkpoint /data/run/best.pt --output /data/candidate.zip
```

Prediction publishes one complete ZIP through a same-filesystem atomic no-clobber
link. It contains bounded finite resource bytes and a hash-bound report, **not** a
HEIC carrier. A failed staging operation publishes nothing and does not modify
inputs. Source and decoded-sidecar identities are reported without fabricated
hardware, person, scene or capture fields.

Only load provenance-checked checkpoints. Loading uses `weights_only=True` and
hashes the same checkpoint bytes it loads; restricted unpickling is not a sandbox.

## Preserved model and evidence

[The model card](models/UniversalPhotographicStyleStateNet.model-card.en.md)
distinguishes byte identity from historical, privately obtained measurements. The
`.mlpackage` is unchanged research evidence. It is not selected by the Rust runtime,
and the removed Swift experimental environment switch is not a supported API.

Run the research regressions with:

```bash
python -m unittest discover -s Tests -p 'test_*.py' -v
```

Tests cover model shapes, finite serialization, byte identity, modality masking,
source integrity, dataset leakage, calibration-only selection and publication
failure. Synthetic test data proves those contracts, not native reconstruction
accuracy. Real-device Photos import/edit/save/reopen evidence remains absent.

## Primary references

- [PyTorch finite gradient checks](https://docs.pytorch.org/docs/stable/generated/torch.nn.utils.clip_grad_norm_.html): reject non-finite gradients rather than scaling them into invalid checkpoints.
- [PyTorch checkpoint loading](https://docs.pytorch.org/docs/stable/generated/torch.load): restricted loading and provenance cautions.
- [Core ML compute units](https://developer.apple.com/documentation/coreml/mlmodelconfiguration/computeunits): permitted processing units do not identify actual ANE placement.
- [scikit-image 0.24 image provenance](https://scikit-image.org/docs/0.24.x/api/skimage.data.html): revision-specific source/license records.

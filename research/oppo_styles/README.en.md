# Research on Photographic Styles reconstruction

English | [简体中文](README.md)

This directory contains models and tools that predict Photographic Styles resources. It is a research workspace. The Rust product does not select these models as a conversion provider.

Predicted `key1`, global tone curve (GTC), light maps, and capture scalars are candidate data. They are not recovered capture facts. Offline error, a parser pass, or an uncertainty threshold does not prove native rendering accuracy or reversible editing in Apple Photos.

## Find the implementation

| File | Responsibility |
| --- | --- |
| [training.py](training.py) | Train single-image models and models with optional, masked input types. |
| [inference.py](inference.py) | Read inputs, check their identities, and produce finite candidate resources. |
| [public_pretraining.py](public_pretraining.py) | Train on public images with analytically generated affine labels. These are not Apple producer labels. |
| [calibration.py](calibration.py) | Fit a shared correction, then apply its frozen coefficients without reading heldout labels. |
| [apple_reverse_key1_training.py](../../xdremux_py/apple_reverse_key1_training.py) | Provide shared training operations for paired images and single-image self-pairs. |

These files do not implement another product converter. The [experiment methodology](methodology.en.md) records historical cohorts, controls, failed attempts, and requirements for renewed experiments.

## Input contracts

The primary image is oriented RGB with shape `3 × 256 × 256`. Integer RGB values use `[0, 255]`. Floating-point values must use `[0, 1]`. The data type determines the range. For example, a byte image whose maximum is 1 remains a dark image.

The model derives nine channels from the primary image. They are the three RGB channels, luma, Cb, Cr, log-luma, and two gradients.

Optional sidecars supply additional decoded inputs:

| Sidecar | Required values | Evidence recorded |
| --- | --- | --- |
| Linear RGB | Oriented scene-linear RGB in `[0, 1]`. | Availability mask and SHA-256 identity. |
| Gain Map | Positive linear exposure ratios; 1 is neutral. | Availability mask and SHA-256 identity. |

A DNG embedded preview is an RGB image, not decoded RAW. A marker string is not a decoded Gain Map. The legacy feature names `has_raw` and `has_gain_map` indicate supplied decoded inputs. File extensions and marker searches do not set these features. The preserved private checkpoint did not learn positive examples for every optional input type.

Metadata has a separate mask for missing values. Boolean EXIF values do not become numeric capture measurements. A missing scalar target contributes neither a loss nor an update to task uncertainty. Its evaluation metric is `null`, not zero.

## Dataset and evaluation contracts

The universal dataset v2 records the identities of the source manifest, extracted label archive, and each cached sample. Validation checks the bytes that the loader decodes. It also checks array shapes, masks, and finite values. Repeated source identities, invalid label indices, and capture-session overlap between splits cause failure.

Preparation checks source identities before and after metadata extraction. Reprepare a legacy v1 manifest from verified inputs. Do not fill missing identities without checking the original bytes.

GTC supervision retains the complete 516-byte Tag3 resource. This resource has a mixed binary layout; it is not a flat Float16 array. A prediction with 516 bytes does not prove that its structural fields form a valid native GTC.

The `base`, `multiscale_large`, and `multimodal_large` variants remain research alternatives. Warm starts must contain every learned parameter and retain target-corpus statistics. New input channels start at zero.

`--resume` continues weights from the same manifest. It does not restore the optimizer or random-number-generator state. Every run, including a continuation, requires a new output directory. The universal trainer rejects existing run directories and output symlinks.

Non-finite predictions, losses, gradients, or selection metrics stop a run. Failed selection cannot reuse an older `best.pt`. The trainer writes checkpoints through temporary files in the new run directory. It pins the manifest identity before fitting and checks that identity before it publishes results.

Calibration selects the checkpoint. Heldout evaluation starts after selection. `consumerProxyRMSE8` uses squared pixel residuals, not squared per-image mean absolute errors. The encoded-RGB quadratic proxy is a training regularizer; it is not Apple's renderer.

## Public pretraining

Public pretraining requires at least three independent source images. It fixes source-disjoint training, calibration, and heldout partitions before fitting. Heldout evaluation follows the selection of fixed weights.

Some historical experiments selected checkpoints with a set named "heldout". Those figures measure the selection set. They are not independent heldout evidence.

The collector records publisher-provided license metadata and attribution. Its allowlist does not establish the legal status of an image. It checks revision-pinned scikit-image sources by Git blob identity before decoding.

Commons original-file SHA-1 and resized-download SHA-256 describe different bytes. The collector does not compare them as identical content. It hashes downloaded tensors separately and does not overwrite existing corpus receipts or tensors.

## Run an experiment

Run the commands from the repository root. Install the research dependencies first:

```bash
python -m pip install '.[training]'
```

The native manifest, source media, caches, checkpoints, and raw per-image reports are private inputs unless separately published. The paths below are examples. Use verified inputs and new output directories.

Prepare and train the universal model:

```bash
python -m research.oppo_styles.train prepare --native-manifest /data/native/manifest.json --output /data/universal-v2
python -m research.oppo_styles.train train --manifest /data/universal-v2/manifest.json --output /data/run
```

Collect public images and train with synthetic labels:

```bash
python -m research.oppo_styles.pretrain collect --curated-only --output /data/public.json --image-directory /data/public-images
python -m research.oppo_styles.pretrain train --manifest /data/public.json --output /data/synthetic-run
```

Only load a checkpoint whose provenance has been checked. The universal loader uses `weights_only=True` and hashes the same bytes that it loads. Restricted unpickling is not a sandbox.

Create a candidate archive:

```bash
python -m research.oppo_styles.predict --input /data/photo.heic --checkpoint /data/run/best.pt --output /data/candidate.zip
```

Prediction publishes one complete ZIP through an atomic hard link on the same filesystem. It refuses to replace an existing destination. A failure during preparation publishes nothing and leaves the inputs unchanged.

The ZIP contains bounded, finite resource values and a report bound to their hashes. It is not a HEIC carrier. The report identifies the source and decoded sidecars without inventing hardware, person, scene, or capture fields.

## Interpret the evidence

The [UniversalPhotographicStyleStateNet model card](models/UniversalPhotographicStyleStateNet.model-card.en.md) separates package identity from historical private measurements. The preserved `.mlpackage` is research evidence. The removed Swift environment switch is not a supported API.

Run Python regression discovery with:

```bash
python -m unittest discover -s tests -t . -p 'test_*.py' -v
```

Tests cover model shapes, finite serialization, byte identity, input masks, dataset integrity, split leakage, calibration-only selection, and publication failures. Synthetic tests establish these software contracts. They do not establish native reconstruction accuracy. Real-device Photos import, edit, save, and reopen evidence remains absent for these research models.

The [methodology](methodology.en.md) retains negative training results and private-ABI failures. Retired cache-report writers and private Swift product hooks are not maintained entry points.

## Primary references

- [PyTorch finite gradient checks](https://docs.pytorch.org/docs/stable/generated/torch.nn.utils.clip_grad_norm_.html).
- [PyTorch checkpoint loading](https://docs.pytorch.org/docs/stable/generated/torch.load).
- [Core ML compute units](https://developer.apple.com/documentation/coreml/mlmodelconfiguration/computeunits).
- [scikit-image 0.24 image provenance](https://scikit-image.org/docs/0.24.x/api/skimage.data.html).

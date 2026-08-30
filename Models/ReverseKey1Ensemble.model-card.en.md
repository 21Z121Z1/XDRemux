# ReverseKey1Ensemble Model Card

English | [简体中文](ReverseKey1Ensemble.model-card.md)

`ReverseKey1Ensemble` is an optional research model for the Photographic Styles Reverse Key 1 path.

It is not the default style-data producer.

## Activation

A caller must explicitly provide the model path through the research configuration used by the current style pipeline.

Do not present this model as the default `constrained-solver`.

## Model contract

| Item | Value |
| --- | --- |
| Format | Core ML `mlprogram` |
| Weight precision | FP16 |
| Input name | `features` |
| Input shape | `1 × 12 × 256 × 256` |
| Output name | `key1` |
| Output shape | `1 × 34560` |
| Serialized active lattice | `12 × 9 × 8 × 10 × 3` |
| Serialized Key 1 size | 51,840 bytes of Float16 data |
| Core ML compute units | `.all` |

The runtime builds feature channels from styled and unstyled image data and their differences.

The ensemble combines a small profile-conditioned baseline with a larger multiscale candidate. The current candidate blend weight is `0.625`.

`.all` lets Core ML select available compute units. It does not prove that a specific inference ran on the Neural Engine.

## Validation boundary

The model has been evaluated as a research fast path on a limited set of OPPO samples and style-response comparisons.

Those results do not prove:

- bit-exact reproduction of an Apple private producer;
- equal quality on unseen devices, lenses, or capture modes;
- a complete Apple Photos import-edit-save-reopen pass;
- Neural Engine execution for every inference.

If the research model or its proxy path fails, the current research envelope can fall back instead of making the optional model a hard product dependency.

## File identity

The model package contains:

- `Manifest.json`;
- `Data/com.apple.CoreML/model.mlmodel`;
- `Data/com.apple.CoreML/weights/weight.bin`.

Use repository or release hashes when exact model identity is part of an experiment. Do not copy old hash values into a new model card after the model package changes.

## Training and export

The repository includes model export and evaluation scripts under `scripts/`.

Training data and checkpoints are not part of the public model package unless a separate artifact explicitly publishes them.

Research changes to this model must not change the documented default Photographic Styles producer unless the CLI default and production tests change in the same revision.

See the [Apple features guide](../docs/apple-features.en.md) for the current product boundary.

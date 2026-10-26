# ReverseKey1Ensemble model card

English | [简体中文](ReverseKey1Ensemble.model-card.md)

`ReverseKey1Ensemble` predicts a Photographic Styles Key 1 resource from a styled image and an unstyled reference. It is a preserved Core ML research model. The Rust CLI does not load this package, and the model is not the default `constrained-solver`.

## Model interface

| Item | Value |
| --- | --- |
| Format | Core ML `mlprogram` |
| Weight precision | FP16 |
| Input | `features`, shape `1 × 12 × 256 × 256` |
| Output | `key1`, shape `1 × 34560` |
| Padded output lattice | `12 × 12 × 8 × 10 × 3` |
| Serialized active lattice | `12 × 9 × 8 × 10 × 3`, or `9 × 12 × 8 × 10 × 3` for portrait orientation |
| Serialized Key 1 | 25,920 Float16 values, or 51,840 bytes |
| Preserved candidate blend weight | `0.625` |

The [feature builder](../../../xdremux_py/apple_reverse_key1_training.py) combines styled RGB, unstyled RGB, their RGB difference, and their YCbCr difference. The ensemble combines a small profile-conditioned baseline with a larger multiscale candidate.

The output tensor includes padding. The [Core ML prediction script](../tools/predict_reverse_key1_coreml.py) selects the active lattice from the displayed image orientation before serialization.

## Run the research predictor

The predictor requires macOS, `coremltools`, and the research Python dependencies. Supply both the styled image and its unstyled reference. The script requests `ComputeUnit.ALL`. This permits Core ML to choose compute units but does not prove Neural Engine execution.

Use new paths for both output files. This diagnostic writes the Key 1 payload and JSON report directly and can overwrite existing files.

Run from the repository root:

```bash
python -m research.oppo_styles.tools.predict_reverse_key1_coreml \
  --model research/oppo_styles/models/ReverseKey1Ensemble.mlpackage \
  --styled /data/styled.png \
  --unstyled /data/unstyled.png \
  --output /data/key1.bin \
  --report /data/key1-report.json
```

The result is a Key 1 payload and a diagnostic report. It is not a converted HEIC. `--reference-key1` adds a numerical comparison with a supplied reference payload. The script does not provide a product fallback when inference fails.

## Package identity

These hashes identify the preserved package bytes:

| Package file | SHA-256 |
| --- | --- |
| `Manifest.json` | `8be470b08161e63d533dc28d9bb500226961adf0db2c1b4cd0c4e9d3b36ea6ef` |
| `Data/com.apple.CoreML/model.mlmodel` | `2759cc216f91ebe5fd5e1e5c3ba55c4d714080435a7b21f91efac0894ecd6a2a` |
| `Data/com.apple.CoreML/weights/weight.bin` | `88bb5c96b5a1e18a818267bed9cc7c0a0a998196ddc073c9c85b1f2507697f2c` |

If the model changes, record new hashes and evaluate the new package separately. Do not copy these identities onto a replacement package.

## Evidence limits

Historical research evaluated this model on a limited set of OPPO images and style-response comparisons. Those observations do not establish:

- byte-exact reproduction of an Apple private producer;
- equal accuracy on unseen devices, lenses, or capture modes;
- complete Apple Photos import, edit, save, and reopen behavior;
- Neural Engine execution for every prediction.

Training data and checkpoints are not part of the public package unless a separate artifact publishes them. Export and evaluation scripts are in [tools](../tools/). Their presence does not make model selection a supported Rust CLI option.

Product integration requires changes to the canonical implementation, documented defaults, and production tests. The [Apple features guide](../../../docs/apple-features.en.md) defines the current product boundary.

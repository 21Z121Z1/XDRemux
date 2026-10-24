# UniversalPhotographicStyleStateNet model card

English | [简体中文](UniversalPhotographicStyleStateNet.model-card.md)

`UniversalPhotographicStyleStateNet` is a preserved Core ML `mlprogram` for research on single-image style reconstruction. The Rust conversion path does not select it. The former Swift switch `XDREMUX_RESEARCH_UNIVERSAL_STYLE_COREML_MODEL` is retired.

The model predicts candidate Key 1, GTC, light maps `c` and `d`, and scalar values. These predictions are not verified capture facts.

## Package interface

The original package record specifies FP16 weights and 10,501,091 trained parameters.

| Direction | Name | Shape and meaning |
| --- | --- | --- |
| Input | `features` | `1 × 9 × 256 × 256` primary-image features. |
| Input | `metadata` | `1 × 16` metadata values. |
| Input | `metadata_mask` | `1 × 16` availability mask. |
| Output | `key1` | `1 × 34560`, padded lattice `12 × 12 × 8 × 10 × 3`. |
| Output | `key1_log_variance` | `1 × 240` uncertainty values. |
| Output | `gtc` | `1 × 516` normalized byte predictions. |
| Output | `light_maps` | `1 × 2048` light-map values. |
| Output | `scalars` | `1 × 6` scalar predictions. |

The six scalar positions are TagH, IOriginalRangeMin, IOriginalRangeMax, IGain, Tag4, and Tag5. Serialization selects a `12 × 9` or `9 × 12` active lattice according to orientation. The resulting Key 1 candidate contains 51,840 bytes.

| Package file | SHA-256 |
| --- | --- |
| `Manifest.json` | `c31a42263e9e23a378edc04371340794a6e27d9425d9755e9892460d217cd7be` |
| `Data/com.apple.CoreML/model.mlmodel` | `84c51a998dc293165ec202215bce8d0412e46776d9a5e4b28aa87cdc13798d4c` |
| `Data/com.apple.CoreML/weights/weight.bin` | `7acd3ed2478aa28e90870140d5c1aaeaa9d929acbe869490c48419495fe7107a` |

Byte-identity checks establish which package was tested. [CoreMLProbe.swift](../CoreMLProbe.swift) uses `.cpuOnly` to check compilation, feature shapes, and finite outputs on synthetic input. It does not measure native style accuracy or Neural Engine placement. Historical runs that permitted `.all` also do not prove Neural Engine execution.

## Historical dataset and accuracy

The measurements below come from the original model card. They have not been reproduced from the public package alone.

The original dataset contained 603 usable iPhone samples across 472 capture sessions. It reported session-disjoint splits with 417 training, 89 calibration, and 97 heldout samples. The devices were iPhone 16, 16 Pro, 17, and 17 Pro.

Private source media, caches, the training checkpoint, and per-image reports are absent from Git. The package cannot independently establish the reported split or its provenance.

| Historical metric | Reported value |
| --- | ---: |
| Universal model heldout Key 1 normalized MAE | `0.82233` |
| Untrained identity Key 1 normalized MAE | `0.93275` |
| Paired styled-and-unstyled ensemble Key 1 normalized MAE | `0.78123` |
| Light-map MAE | `0.52812` |
| Scalar MAE | `0.42440` |
| Auxiliary unstyled-image MAE | `0.07256` |

These observations support retention of a single-input research alternative. They do not establish a replacement for the more accurate paired model or native equivalence on arbitrary images.

## Historical OPPO coverage

A trial without native labels reported 213 distinct originals from Find X6 Pro, X7 Ultra, X8 Ultra, X9, and X9 Ultra. The set contained 141 HEIC, 62 JPEG, and 10 DNG files, with no reported decoding failures.

A threshold derived from iPhone calibration uncertainty admitted 187 of 213 candidates (`87.8%`). This is a coverage result. It does not measure Apple rendering accuracy.

The former card also reported PNG, TIFF, WebP, and AVIF decoding trials. DNG decoding used an embedded preview but incorrectly marked RAW as available. Maintained inference sets input availability only when a decoded sidecar is supplied. No CIRAW linear tensor was connected to the published checkpoint. The historical DNG result therefore does not demonstrate RAW-conditioned inference.

## Historical timing and conversion comparison

| Observation | Reported value |
| --- | ---: |
| Local MPS model p95 | `29.2 ms` |
| Local MPS end-to-end p95 | `0.693 s` |
| Local MPS end-to-end maximum | `1.066 s` |
| Core ML warm p95 on one X9 Ultra HEIC | `21.9 ms` |
| Mean absolute Key 1 conversion discrepancy on that input | `9.04e-5` |
| Maximum Key 1 conversion discrepancy on that input | `0.001175` |

These timings depend on the test machine. The single-input conversion comparison does not establish general latency, accuracy, or execution on the Neural Engine.

## Unverified behavior

No cited historical trial completed Apple Photos import, edit, save, and reopen. The evidence does not show that admitted candidates outperform identity or the constrained solver in native rendering.

Finite candidate generation, native framework parsing, native rendering, and reversible Photos editing are separate claims. A pass at one level does not establish the next level. See the [research entry point](../README.en.md) for maintained tools and the [methodology](../methodology.en.md) for experiment controls.

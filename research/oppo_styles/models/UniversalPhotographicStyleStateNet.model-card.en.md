# UniversalPhotographicStyleStateNet — research candidate

English | [简体中文](UniversalPhotographicStyleStateNet.model-card.md)

This preserved Core ML `mlprogram` is not a product provider. The former Swift
`XDREMUX_RESEARCH_UNIVERSAL_STYLE_COREML_MODEL` environment switch is retired;
canonical Rust conversion does not consult it. Candidate GTC, c/d and scalars are
predictions, not verified capture facts.

## Byte contract

The package reports FP16 weights and 10,501,091 trained parameters. `features` is
`1 × 9 × 256 × 256`; `metadata` and `metadata_mask` are each `1 × 16`. Outputs are
`key1` (`1 × 34560`, padded `12 × 12 × 8 × 10 × 3`), `key1_log_variance` (`1 × 240`),
`gtc` (`1 × 516` normalized bytes), `light_maps` (`1 × 2048`) and `scalars` (`1 × 6`).
The six scalar positions are TagH, IOriginalRangeMin, IOriginalRangeMax, IGain,
Tag4 and Tag5. Only an orientation-appropriate `12 × 9` or `9 × 12` lattice is
serialized into a 51,840-byte key1 candidate.

| Package file | SHA-256 |
| --- | --- |
| `Manifest.json` | `c31a42263e9e23a378edc04371340794a6e27d9425d9755e9892460d217cd7be` |
| `Data/com.apple.CoreML/model.mlmodel` | `84c51a998dc293165ec202215bce8d0412e46776d9a5e4b28aa87cdc13798d4c` |
| `Data/com.apple.CoreML/weights/weight.bin` | `7acd3ed2478aa28e90870140d5c1aaeaa9d929acbe869490c48419495fe7107a` |

Automated byte-identity and synthetic-forward tests are separate from historical
accuracy and timing observations below. Allowing `.all` compute units does not
prove Neural Engine execution.

## Historical observations, not newly reproduced measurements

The original model card reported 603 usable native iPhone samples across 472
capture sessions: 417 train, 89 calibration and 97 heldout samples, covering
models 16, 16 Pro, 17 and 17 Pro. It reported session-disjoint splits. The underlying
private media, caches, training checkpoint and per-image reports are not in Git,
so those statistical and provenance assertions cannot be independently rerun from
this package alone.

Reported heldout key1 normalized MAE was `0.82233`, versus untrained identity
`0.93275` and the better paired styled+unstyled ensemble `0.78123`. Reported light-map
MAE was `0.52812`, scalar MAE `0.42440` and auxiliary unstyled MAE `0.07256`. These
figures justify preserving a single-input research alternative, not replacing
the more accurate paired path or claiming arbitrary-image native equivalence.

The label-free OPPO trial reported 213 distinct originals from Find X6 Pro,
X7 Ultra, X8 Ultra, X9 and X9 Ultra: 141 HEIC, 62 JPEG and 10 DNG; no decoding
failures. A threshold derived from iPhone calibration uncertainty admitted
187/213 (`87.8%`) candidates. That is coverage, not Apple-response accuracy.

Historical local MPS timings were model p95 `29.2 ms`, end-to-end p95 `0.693 s`
and maximum `1.066 s`. A Core ML trial on one X9 Ultra HEIC reported warm p95
`21.9 ms`, mean absolute key1 conversion discrepancy `9.04e-5` and maximum
`0.001175`. Machine-specific timings and single-input parity are not a general
latency, quality or ANE-placement guarantee.

The former card reported additional PNG/TIFF/WebP/AVIF decoding trials. DNG used
an embedded preview but incorrectly marked RAW as available. Maintained inference
now marks a modality available only when a decoded sidecar was supplied. No CIRAW
linear tensor was connected to the published checkpoint. Consequently the old DNG
coverage result is not evidence of RAW-conditioned inference.

No cited historical trial completed Apple Photos import, edit, save and reopen.
No evidence establishes that the admitted candidates outperform identity or a
constrained solver on native response. Finite candidate generation, native
framework parsing, native response and reversible Photos editing remain different
evidence levels.

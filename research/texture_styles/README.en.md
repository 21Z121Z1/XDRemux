# Texture Style research

English | [简体中文](README.md)

**Research only.** The Rust product does not fabricate Texture Style support. This directory preserves native-contract observations, rejected shortcuts, synthetic probes and explicit promotion gates. It is not a second converter and does not write HEIF files. The [dated convergence ledger](../../docs/convergence/2026-09-23/README.en.md) identifies the historical commits and every retired artifact; its archival refs are provenance, not development branches.

## What the carrier experiments established—and did not

The historical matrix varied three independent layers: the 2023 Styles resource, the 2026 Texture metadata item, and Apple MakerNote tag 84. Treating any one layer's parser result as a complete editable-photo contract was incorrect. The experiments also varied item placement (`idat`/method 1 versus `mdat`/method 0), metadata item names, the global `mdat` layout and semantic resources. A placement result for one tested carrier is not a universal requirement of HEIF or every native reader.

The useful structural vocabulary is:

| Resource | Observed/hypothesized contract | Limit |
| --- | --- | --- |
| Existing Styles | `tag:apple.com,2023:photo:metadata:styles`, binary plist and `cdsc` relationship to primary/tone-map resources | Native v8, generated v15, and experimental v16 cannot be conflated. The product keeps its verified producer format. |
| Texture metadata | `tag:apple.com,2026:photo:metadata:texture_styles`; experiments used item name `metadata` | A matching URI or item name is not evidence of valid semantics. Select by exact identity/relationship, never largest payload or first URI-like item. |
| Texture information | `Preset`, `CaptureType`, `CaptureMode`, `PortType`, `HardwareModel`, `TextureStylePeopleDataVersion`, `FilmGrainSeed`, optional postprocessed people data | These include capture facts. They cannot be copied from a donor or filled with iPhone defaults for an OPPO source. No-person variants omitted postprocessed people data rather than inventing people. |
| Apple MakerNote | Preserve its byte order, unknown fields and source `PhotoIdentifier` (tag 43); tag 84 is a typed binary plist | Boolean, integer, Float32 and Float64 wire representations are not interchangeable merely because decoded Python values compare equal. A missing Apple MakerNote is not permission to invent one. |
| Semantic resources | Role-specific auxiliary image, descriptor, `auxC` identity, `ipma` associations and `auxl`/`cdsc` edges | Repeated labels or parser-visible resources do not prove role-specific inference. Per-person instance resources are not a thirteenth interchangeable category mask. |

The twelve added category URIs use the `tag:apple.com,2026:photo:aux:` prefix with `semanticnosematte`, `semanticskinmattev2`, `semanticnonfaceskinmatte`, `semanticlipsmatte`, `semanticteethmattev2`, `semanticpersonmatte`, `semanticglassesmattev2`, `semanticeyebrowsmatte`, `semantictattoomatte`, `semantichandsmatte`, `semanticearsmatte`, and `semanticfaceskinmatte`. Their exact role-to-producer mapping remains a promotion requirement. A category enumeration from a private Vision result is not itself that mapping.

`evidence/native-standard-matrix.json` preserves the historical B–E candidate definitions byte-for-byte. In particular, the marketing-name, `iPhone19,2`, and `iPhone19,3` alternatives are **candidate hypotheses**, not verified capture facts for arbitrary input. The file is never consumed by the product or by a writer. The retired validator checked decoded values, a single extent, and structural relationships; it did not establish wire-type fidelity, all location semantics, or Photos editability.

## Rejected carrier promotion

The historical Rust carrier built a v16 Styles plist, then aliased the same 2019 skin image and MIME descriptor into all twelve 2026 roles. It synthesized a fixed 216-byte Texture plist with iPhone hardware/capture claims, a fixed 133-byte tag-84 plist, and—when absent—a new Apple MakerNote with a source-hash-derived UUID-shaped identifier. Deriving an identifier from source bytes does not turn fabricated Apple capture metadata into a source fact. “No donor image bytes” also does not prove the correctness of a semantic label.

Those automatic product hooks, metadata templates and construction helpers are intentionally retired. Their historical source remains recoverable through the ledger's annotated archive tags. Their valid structural lessons are retained here: preserve unrelated payloads/unknown boxes and layout when isolating one variable; resolve all extents with bounds/overflow checks; retain existing identifiers; and validate the **same final bytes** that are atomically published. Current Rust format/HEIF/runtime owners, not a revived carrier backend, own those invariants.

Blank independently encoded masks, same-source aliases and 13-way resource-presence scans were useful negative controls. They are not native inference. Equal hashes are not universally invalid either: two genuinely inferred absent categories can both be empty. Promotion needs producer identity and role meaning, not merely unequal hashes. The native-shaped MakerNote refinement and the final-publication test did not remove this semantic blocker. The publication invariant is retained in the current lifecycle tests without depending on a historical local variable name.

## Person input and ROI experiment

`PersonInputProbe.m` retains the nine synthetic cases: CGRect dictionary versus NSValue, optional yaw/pitch/roll, four landmark-key alternatives and the all-aliases control. The synthetic face is normalized `(0.20, 0.25, 0.40, 0.40)` with face ID 17; the 76 ellipse landmarks are deliberately artificial. The optional crop matrix isolates the native `normalizePersonInputDataArray:toCropRect:` transformation. It does not claim that a face ROI and a skin ROI are identical or recover a native image inference model.

The probe resolves `kFigCaptureStreamMetadata_*` keys dynamically and records which resolutions are real and which remain fallback-string hypotheses. It observes `faceROI`, `faceSkinROI`, `faceROIAndLandmarksROIRelativeScalingROI`, pose/landmark fields and instance references. It checks argument count, object/CGRect types and return types **before** `NSInvocation` writes or reads argument storage. Unknown ABI, a missing requested normalization method, exception, nil/non-array result, or missing person produces a failure, not a successful empty report. Each case runs in its own bounded child process, so an abort does not erase the rest of the matrix.

`InputTrace.m` observes the input/output of `personInputDataArrayFromDetectedFaces:`, `personInputDataFromStillProperties:`, `setInputPersonData:` and `setInputSkinSmoothingFaceDetections:`. It hooks only owned methods with the expected ABI and always forwards to the original implementation. Logging failures do not suppress that call. The trace is opt-in for an owned local research harness; it is not installed into Photos or a system process by CI.

```sh
# Each command needs a new directory whose parent already exists.
python -m research.texture_styles.probe self-test --output /tmp/texture-abi-check
python -m research.texture_styles.probe person --output /tmp/texture-person-matrix \
  --normalize 0.03 0.04 0.94 0.92
python -m research.texture_styles.probe runtime --output /tmp/texture-runtime-inventory
```

`self-test` compiles with the public macOS SDK and checks the ABI guard against a synthetic NSObject class; it invokes no private selector. The research CI runs that mode. `person` and `runtime` are explicit local research operations; unavailable frameworks/methods are not product failures. Reports retain source hashes, compiler/SDK, OS/architecture, each command, exit code, timeout and log hashes. An incomplete matrix exits nonzero and remains marked incomplete. The wrapper never downloads firmware or accepts a private photo.

A trace may be loaded by an owned non-hardened harness with `dlopen`, or explicitly through `DYLD_INSERT_LIBRARIES`, `XDREMUX_TEXTURE_TRACE=1`, and `XDREMUX_TEXTURE_TRACE_FILE`. Do not disable SIP, weaken a signed application's validation, or upload private traces/media. CI compiles the trace but does not inject it.

## FSINC / E5 exploration and corrected sparse oracle

Historical local probes explored ANSTKit's FSINC configuration, descriptor, E5 network wrapper and instance/category mask APIs. Version candidates were `0x2012c` (2.3) and `0x20190` (2.4); resolution candidates were 0 = 768×576, 1 = 576×768, and 2 = 256×256. The old image probe used centered aspect-fill BGRA/sRGB input. Its call order was concrete-class selection → configuration → algorithm → `prepareWithError:` → input binding → inference → instance/category masks. Trying compute-device integers 0–7, exporting category numbers, or printing a non-nil wrapper was not proof of supported ANE execution or the correct twelve semantic roles.

Further experiments compared macOS 26/Xcode 27 environments, descriptor factories, E5 preparation, MIL version/opset namespaces, `constexpr_*` sparse operators, Core ML loading and model/blob readers. These are distinct hypotheses. Merely renaming an opset or header cannot supply a missing compiler operator, decode proprietary packed blobs, or prove model parity. Historical workflows included permissive diagnostic steps and binary dumps; their green job result must not be reinterpreted as successful inference. No such acquisition, memory-dump or one-shot runner workflow is restored. `RuntimeInventory.m` replaces unbounded raw-memory/ivar reads with a read-only inventory of relevant class declarations, method encodings and symbol availability. It does not call a guessed native function ABI.

One historical r10 identity was demonstrably wrong for nonzero offsets:

```text
wrong: sparse_shift_scale_then_dense(mask, q, scale, offset)
       == dense_shift_scale(sparse_to_dense(q, mask), scale, offset)

correct: materialized[index] = scale[block] * (q[next] - offset[block])
         only where mask[index] == 1; every other entry remains zero.
```

For mask `[[1,0],[0,1]]`, q `[4,7]`, scale `0.5`, offset `2`, the correct result is `[[1,0],[0,2.5]]`, not `[[1,-1],[-1,2.5]]`. `sparse.py` independently implements the bounded, explicitly unpacked operation. Tests cover block coordinates, both float widths, optional/nonzero offsets, C-order traversal, malformed inputs and overflow. `oracle.py` compares twelve synthetic cases against **coremltools 9.0**'s public operator implementation. This is an operator-equivalence test, not a native FSINC model conversion or inference claim. The exact old materialization recipe is archived with its rejected identity recorded, rather than silently preserved as valid.

## Promotion ladder and evidence limits

A product change requires source-backed capture fields and every advertised semantic role, explicit dimensions/orientation/ROI transforms and resource relationships, finite model outputs, malformed-input rejection, and preservation of primary/Gain Map media. Then it needs final-byte HEIF validation, native framework consumption on the applicable OS/device, Photos controls, edit/save/reopen/re-render/reversible-edit evidence, and failure-safe publication through the current Rust runtime. Missing native inputs must stay missing or fail explicitly; synthetic controls never satisfy the ladder.

Historical device-positive carrier reports support their reported admission observations only. They do not independently establish general editability or role truth. Private sample bytes and deleted/expired Actions outputs are not reconstructed or claimed to have been rerun here. The archive preserves the tracked source and methodology; the current CI and receipts establish only the checks they actually execute. Reopening a hypothesis requires a fresh artifact hash, OS/device/software identity, exact command, expected versus actual observations and a declared promotion gate.

## Primary references

- [NSMethodSignature argument count](https://developer.apple.com/documentation/foundation/nsmethodsignature/numberofarguments): includes hidden `self` and `_cmd`.
- [NSInvocation argument storage](https://developer.apple.com/documentation/foundation/nsinvocation/setargument:atindex:) and [return ownership](https://developer.apple.com/documentation/foundation/nsinvocation/getreturnvalue:): size/type guards precede invocation.
- [Objective-C method replacement](https://developer.apple.com/documentation/objectivec/method_setimplementation(_:_:)): retain the original IMP; do not assume a selector's ABI.
- [Core ML sparse operator source](https://apple.github.io/coremltools/_modules/coremltools/converters/mil/mil/ops/defs/iOS18/compression.html#constexpr_sparse_blockwise_shift_scale): dequantization applies only to mask-selected entries.
- [Pinned public oracle release](https://pypi.org/project/coremltools/9.0/): optional isolated research dependency, not a product dependency.

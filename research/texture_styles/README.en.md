# Texture Style research

English | [简体中文](README.md)

This directory contains historical observations, rejected methods, and diagnostic probes for Texture Styles. **The probes are research tools.** They do not write HEIF files or provide another product converter.

The current Rust product has a separate derived Texture Styles transaction. See [Apple features](../../docs/apple-features.en.md) and the [runtime owner](../../crates/xdremux-runtime/src/apple_texture.rs). That transaction does not claim to recover the complete camera FSINC producer.

The previously referenced convergence ledger is absent from this tree. Verify historical commits and archived source against their original provenance records. This summary does not establish that every old artifact remains available.

## Historical carrier questions

The carrier experiments varied three independent layers: the 2023 Styles resource, the 2026 Texture metadata item, and Apple MakerNote tag 84. A parser accepting one layer did not establish a complete editable-photo contract.

The experiments also varied `idat`/method 1 versus `mdat`/method 0 placement, item names, global `mdat` layout, and semantic resources. A placement result for one candidate does not define a universal HEIF or native-reader requirement.

| Resource | Observed or proposed representation | Interpretation limit |
| --- | --- | --- |
| Existing Styles | `tag:apple.com,2023:photo:metadata:styles`, binary plist, and `cdsc` links to primary/tone-map resources | Native v8, generated v15, and experimental v16 are distinct formats. Keep the verified producer format. |
| Texture metadata | `tag:apple.com,2026:photo:metadata:texture_styles`; experimental item name `metadata` | A matching URI or name does not prove valid semantics. Select the exact identity and relationship, not the largest payload or first similar URI. |
| Texture information | `Preset`, `CaptureType`, `CaptureMode`, `PortType`, `HardwareModel`, `TextureStylePeopleDataVersion`, `FilmGrainSeed`, and optional postprocessed people data | Capture fields must represent source facts. Donor fields and iPhone defaults cannot describe an OPPO capture. No-person candidates omitted people data. |
| Apple MakerNote | Existing byte order, unknown fields, source `PhotoIdentifier` (tag 43), and typed binary-plist tag 84 | Boolean, integer, Float32, and Float64 encodings are distinct even when decoded values compare equal. Missing capture metadata must not be fabricated. |
| Semantic resources | Per-role auxiliary image, descriptor, `auxC` identity, `ipma` associations, and `auxl`/`cdsc` relationships | Labels and parseable resources do not prove inference. A per-person instance resource is not an interchangeable thirteenth category mask. |

The twelve category URI suffixes below use the prefix `tag:apple.com,2026:photo:aux:`. Keep each suffix exact.

| Category URI suffix | Category URI suffix |
| --- | --- |
| `semanticnosematte` | `semanticskinmattev2` |
| `semanticnonfaceskinmatte` | `semanticlipsmatte` |
| `semanticteethmattev2` | `semanticpersonmatte` |
| `semanticglassesmattev2` | `semanticeyebrowsmatte` |
| `semantictattoomatte` | `semantichandsmatte` |
| `semanticearsmatte` | `semanticfaceskinmatte` |

A claim about native category production requires an exact mapping from each role to its producer. Private Vision category numbers alone do not supply that mapping. The current derived product context has narrower semantics: some protection regions come from native landmarks or hand poses. It disables ears and tattoo protection. Its [current feature contract](../../docs/apple-features.en.md) defines those limits.

[`evidence/native-standard-matrix.json`](evidence/native-standard-matrix.json) preserves the historical B–E definitions byte-for-byte. Marketing-name values, `iPhone19,2`, and `iPhone19,3` are candidate hypotheses, not verified capture facts for arbitrary input. Neither the product nor a writer consumes this file.

The retired validator checked decoded values, one extent, and structural relationships. It did not establish wire-type fidelity, every location rule, or Photos editability.

## Rejected promotion method

An earlier Rust carrier generated a v16 Styles plist. It aliased one 2019 skin image and its MIME descriptor into all twelve 2026 roles. It also generated a fixed 216-byte Texture plist with iPhone capture claims and a fixed 133-byte tag-84 plist.

When an Apple MakerNote was absent, that carrier created one with a UUID-shaped identifier derived from the source hash. A source-derived identifier did not make the invented capture fields true. Avoiding donor image bytes did not prove that the semantic labels were correct.

Those automatic hooks, metadata templates, and construction helpers are retired. Any recovery of their historical source requires the original archive records. The remaining structural requirements belong to the current Rust format, HEIF, and runtime owners:

- Preserve unrelated payloads, unknown boxes, and layout while isolating an experimental variable.
- Check bounds and overflow for every extent.
- Preserve existing identifiers.
- Validate the same final bytes that the runtime publishes atomically.

Blank independently encoded masks, same-source aliases, and 13-way resource-presence scans were negative controls. They did not demonstrate native inference. Equal hashes are not always wrong: two genuinely inferred absent categories can both be empty.

Producer identity and role meaning are required evidence. Different hashes alone are insufficient. MakerNote refinement and final-publication tests did not remove that semantic gap. Current lifecycle tests retain the publication requirement without depending on a historical local variable name.

## Person input and region-of-interest probes

[`PersonInputProbe.m`](PersonInputProbe.m) defines nine synthetic cases. They compare CGRect dictionaries with NSValue, optional yaw/pitch/roll, four landmark-key alternatives, and an all-aliases control. The synthetic face rectangle is `(0.20, 0.25, 0.40, 0.40)` in normalized coordinates. Its face ID is 17, and its 76 ellipse landmarks are artificial.

The optional crop experiment isolates `normalizePersonInputDataArray:toCropRect:`. It does not establish that a face region of interest (ROI) equals a skin ROI. It does not recover an image inference model.

The probe resolves `kFigCaptureStreamMetadata_*` keys dynamically. Its report distinguishes resolved symbols from fallback-string hypotheses. It observes `faceROI`, `faceSkinROI`, `faceROIAndLandmarksROIRelativeScalingROI`, pose/landmark fields, and instance references.

Before `NSInvocation` accesses argument or return storage, the probe checks argument count, object/CGRect types, and return types. Unknown ABI, a missing requested normalization method, an exception, nil/non-array output, or a missing person causes failure. Each case has a separate child process and deadline. A process abort does not erase the other cases.

[`InputTrace.m`](InputTrace.m) observes these methods:

- `personInputDataArrayFromDetectedFaces:`
- `personInputDataFromStillProperties:`
- `setInputPersonData:`
- `setInputSkinSmoothingFaceDetections:`

The trace replaces only methods owned by the target class with the expected ABI. It always forwards to the original implementation, including after logging failure. It is opt-in for an owned local research harness. CI does not inject it into Photos or a system process.

### Run the probes

Use macOS and the Xcode command-line tools. Each command needs a new output directory whose parent already exists.

```sh
python -m research.texture_styles.probe self-test --output /tmp/texture-abi-check
python -m research.texture_styles.probe person --output /tmp/texture-person-matrix \
  --normalize 0.03 0.04 0.94 0.92
python -m research.texture_styles.probe runtime --output /tmp/texture-runtime-inventory
```

`self-test` uses the public SDK and a synthetic NSObject class to check the ABI guard. It invokes no private selector. The research workflow runs this mode.

`person` and `runtime` are explicit local experiments. An unavailable framework or method is an experiment limitation, not a product failure. Reports retain source hashes, compiler/SDK, OS/architecture, commands, exit codes, timeouts, and log hashes. An incomplete matrix remains marked incomplete and exits nonzero. The wrapper neither downloads firmware nor accepts private photos.

An owned harness without hardened-runtime restrictions can load the trace with `dlopen`. An explicit alternative uses `DYLD_INSERT_LIBRARIES`, `XDREMUX_TEXTURE_TRACE=1`, and `XDREMUX_TEXTURE_TRACE_FILE`. Do not disable SIP, weaken a signed application's validation, or upload private traces/media. CI compiles the trace without injecting it.

## FSINC and E5 observations

Historical probes explored ANSTKit FSINC configuration, descriptors, an E5 network wrapper, and instance/category mask APIs. The candidates were:

| Parameter | Historical candidates |
| --- | --- |
| Version | `0x2012c` (2.3); `0x20190` (2.4) |
| Resolution | 0 = 768×576; 1 = 576×768; 2 = 256×256 |
| Compute-device value | Integers 0–7 |
| Image input | Centered aspect-fill BGRA/sRGB |

The call order was class selection, configuration, algorithm creation, `prepareWithError:`, input binding, inference, and instance/category mask retrieval. Trying a compute-device integer did not establish supported Apple Neural Engine (ANE) execution. Category numbers and non-nil wrappers did not establish the twelve semantic roles.

Further experiments covered macOS 26/Xcode 27, descriptor factories, E5 preparation, MIL versions/opsets, `constexpr_*` sparse operators, Core ML loading, and model/blob readers. These were separate hypotheses. Renaming an opset or header cannot supply a missing compiler operator, decode proprietary packed data, or prove model equivalence.

Some old workflows allowed diagnostic failures and produced binary dumps. A green job did not establish successful inference. The current directory does not restore those acquisition, memory-dump, or one-shot workflows. [`RuntimeInventory.m`](RuntimeInventory.m) reads class declarations, method encodings, and symbol availability. It does not read arbitrary raw memory or call a guessed native function ABI.

## Correct sparse operation

The historical r10 identity was wrong when offsets were nonzero:

```text
wrong: sparse_shift_scale_then_dense(mask, q, scale, offset)
       == dense_shift_scale(sparse_to_dense(q, mask), scale, offset)

correct: materialized[index] = scale[block] * (q[next] - offset[block])
         only where mask[index] == 1; every other entry remains zero.
```

For mask `[[1,0],[0,1]]`, q `[4,7]`, scale `0.5`, and offset `2`, the correct result is `[[1,0],[0,2.5]]`. The result `[[1,-1],[-1,2.5]]` is wrong.

[`sparse.py`](sparse.py) implements the bounded operation with explicitly unpacked input. Tests cover block coordinates, both float widths, optional/nonzero offsets, C-order traversal, malformed input, and overflow.

[`oracle.py`](oracle.py) compares twelve synthetic cases with the public operator in **coremltools 9.0**. This checks operator equivalence only. It does not convert a native FSINC model or establish native inference. The historical materialization recipe retains its rejected identity as evidence of the error.

## Requirements for further product changes

A product change must document source-backed capture fields and the producer of each advertised semantic role. It also needs explicit dimensions, orientation, ROI transforms, resource relationships, finite outputs, malformed-input rejection, and preservation of primary/Gain Map media.

Validate the final HEIF bytes and native consumption on the applicable OS/device. Then verify Photos controls, editing, saving, reopening, rendering, and reversibility. Publication must use the current Rust runtime and preserve existing outputs on failure. Missing native inputs must remain absent or cause explicit failure. Synthetic controls cannot replace native evidence.

Historical positive device reports support only the admission behavior that they recorded. They do not independently establish general editability or role truth. Private sample bytes and deleted or expired Actions outputs have not been reconstructed or rerun for this document.

To reopen a hypothesis, record a fresh artifact hash, OS/device/software identity, exact command, expected result, actual result, and acceptance requirement. Current CI and reports establish only the checks they execute.

## Literal historical evidence

[`evidence/historical-contracts.json`](evidence/historical-contracts.json) retains seven small RC/numeric-key/candidate records with original commit, blob, and SHA-256 identities. Its original statements and synthetic values remain historical hypotheses. They are not current capture facts.

The records preserve experiment definitions without keeping competing patchers or reacquiring private frameworks and firmware. `native-standard-matrix.json` remains byte-exact. Its duplicate in the provenance index is historical evidence, not another runtime configuration.

## Primary references

- [NSMethodSignature argument count](https://developer.apple.com/documentation/foundation/nsmethodsignature/numberofarguments): includes `self` and `_cmd`.
- [NSInvocation argument storage](https://developer.apple.com/documentation/foundation/nsinvocation/setargument:atindex:) and [return ownership](https://developer.apple.com/documentation/foundation/nsinvocation/getreturnvalue:): check size and type before invocation.
- [Objective-C method replacement](https://developer.apple.com/documentation/objectivec/method_setimplementation(_:_:)): retain the original IMP and verify the selector ABI.
- [Core ML sparse operator source](https://apple.github.io/coremltools/_modules/coremltools/converters/mil/mil/ops/defs/iOS18/compression.html#constexpr_sparse_blockwise_shift_scale): dequantization applies only to selected entries.
- [Pinned public oracle release](https://pypi.org/project/coremltools/9.0/): optional research dependency, not a product dependency.

# Dual-layer HDR interoperability work plan

This plan tracks the explicit BT.2020 dual-layer profiles and the remaining export extensions. The supported CLI behavior and its limits are in [the CLI reference](../cli.en.md).

## Outcome

Extend the existing Rust media pipeline to produce and inspect richer dual-layer HDR files while retaining the current ISO Gain Map and OPPO Gallery behaviors. Keep the compressed SDR base unchanged whenever the operation only repackages its HDR resources. A new profile must be selected explicitly and must fail closed when the source cannot support its declared color or metadata semantics.

## Implementation

### 1. Profile and source facts

- Add an explicit export/validation profile to the Rust engine, separate from the existing output intent and vendor compatibility decisions.
- Inventory the source base image, Gain Map, ICC/CICP, dimensions, orientation, numeric metadata, and any verified HDR mastering or dynamic metadata.
- Plan the transformation from source facts and supported codec capabilities. Do not infer an HDR color encoding from the output filename, a Gain Map's channel count, or the SDR ICC alone.
- Keep the current standard output and OPPO-compatible paths unchanged by default. Do not duplicate product policy in the CLI, runtime, Swift adapter, or research scripts.

### 2. Metadata representation

- Extend xdremux-metadata with typed, bounded models for optional HDR static metadata, dynamic metadata, and additional color mapping data.
- Preserve the existing Gain Map metadata semantics: one or three component records, exact source rational values when available, signed gain bounds, gamma, offsets, and headroom.
- Add checked serialization and parsing for optional HEIF it35 data and the JPEG APP8 extension carrier when those output paths are implemented.
- Accept matrix and 3D LUT mappings only when their values, dimensions, coordinate space, and provenance are known. Reject unsupported mandatory mapping semantics rather than silently ignoring them.
- Do not fabricate mastering-display information, peak luminance measurements, or device-specific metadata.

### 3. HEIF container construction

- Reuse xdremux-heif's existing primary/hidden Gain Map/tmap graph, ordered dimg references, auxiliary links, and altr grouping.
- Validate pitm, infe, iloc, iprp/ipma, idat/mdat, colr, pixi, hvcC, dimensions, and item-ID uniqueness as a coherent output graph.
- For the new profile, make orientation properties and image geometry consistent across the base, Gain Map, and derived HDR item. Retain the existing Apple-oriented layout in its current profile.
- Carry validated HDR static metadata in the appropriate item properties and attach optional extension metadata without changing unrelated image resources.
- Preserve existing media and metadata on failure; publish only after final-byte validation.

### 4. Color and HDR reconstruction

- Define base colorimetry, Gain Map application primaries, and derived HDR colorimetry independently.
- Decode into the required linear RGB working space, apply channel gains and offsets, then perform any required gamut conversion and output transfer encoding.
- Support a verified BT.2020 PQ/HLG derived HDR path, including a P3 SDR base where the Gain Map is applied in the base primaries.
- Never replace a color tag in lieu of converting pixels. Do not reinterpret Gain Map compression nclx as its application or output color space.
- Retain the existing SDR pixels and SDR rendering behavior. Do not require a second full-resolution image render solely to repackage metadata.

### 5. JPEG output (later slice)

- Add a dedicated dual-JFIF writer with independently decodable base and enhancement images.
- Write and cross-check MPF image associations and offsets, ISO APP2 Gain Map metadata, ICC/Exif metadata, and the optional APP8 extension payload.
- Keep existing JPEG input and JPEG-to-HEIF behavior unaffected.

### 6. Validation

- Extend xdremux validate with an explicit profile check. Report separate results for container structure, metadata semantics, decoded HDR pixels, SDR preservation, and device interoperability.
- Reject invalid rational denominators, unsupported versions, contradictory ICC/CICP, inconsistent Gain Map channel counts, broken associations, wrong orientation, malformed extensions, and invalid byte ranges.
- Build an independent reference reconstruction that reads the final file and compares actual color and luminance against known inputs. Test without deriving expected values from the writer being tested.
- Distinguish a portable structural pass from Apple ImageIO, Photos, Android Gallery, and physical-display observations.

## Implementation slices

- [x] A. Checked tone-map, static HDR, and registered metadata payloads. Semantic validation reads final bytes and rejects invalid versions, denominators, channel counts, color declarations, lengths, and associations.
- [x] B. Explicit HEIF PQ and HLG profiles with common coded dimensions and ordered rotation/mirror transforms. The existing compatibility layout remains the default.
- [x] C. Relative linear BT.2020 reconstruction from final-file decoded resources, independent numeric references, compressed SDR preservation, and failure preservation checks. Display adaptation is separate.
- [ ] D. Static HDR and registered dynamic carriers round-trip in the Rust assembly API. Encoded dynamic statistics, target parameters, curves, splines, and saturation values are decoded with bounds checks; source bytes are retained. Registered matrix/LUT payloads are decoded and preserved in both HEIF and JPEG. Actual dynamic display adaptation and application of the scalar LUT remain blocked on unambiguous rendering semantics.
- [x] E. Dual-JFIF output has association, offset, and final-file decode tests. The registered reduced-view offset excludes non-JFIF APP0 as well as APP1–APP15. Independent producer interoperability evidence is still needed.
- [x] F. The English and Chinese CLI contracts describe export, semantic validation, decoded reconstruction, and unsupported source combinations. Native/device acceptance remains a separate evidence layer.

## Acceptance criteria

- Existing default conversions produce equivalent SDR and HDR resources under their current contracts; OPPO-compatible output is not silently changed.
- Any newly selected profile yields a structurally and semantically validated file, or stops without publishing a partially valid output.
- P3-to-BT.2020 conversion and PQ/HLG handling are tested against independent numeric references, including high-headroom highlights and sub-unity gains.
- Cover mono/RGB Gain Maps, varying pixel subsampling and bit depth, grid and direct resources, orientation/mirroring, source ICCs, and mixed or missing optional metadata.
- Failed conversions preserve input bytes and any previously published output.
- Portable tests and relevant native/device checks are reported separately and tied to the actual implementation commit.

## Code ownership

- xdremux-engine: source facts, capability gates, profile selection, conversion plan.
- xdremux-metadata: typed metadata, parsers, and serializers.
- xdremux-heif / xdremux-format: container graph, item properties, and structural validation.
- xdremux-hdr: reference reconstruction and color math.
- xdremux-runtime: execution, final-byte validation, and safe publication.
- xdremux-cli: minimal profile selection and machine-readable validation report.

Use the repository's existing tests, fixture integrity rules, and exact-HEAD verification workflow. No new standalone backend or CI framework is needed.

## Validation commands for the current slice

```sh
cargo test --locked -p xdremux-metadata -p xdremux-heif -p xdremux-runtime -p xdremux-cli
cargo clippy --locked --workspace --all-targets -- --deny warnings
cargo run --locked -p xdremux-cli -- validate output.heic --semantic --json
cargo run --locked -p xdremux-cli -- convert --input input.heic --output output.heic --hdr-profile bt2020-pq
cargo run --locked -p xdremux-cli -- validate output.heic --hdr-profile bt2020-pq --decode-hdr --json
```

The semantic check is opt-in. The explicit profile check also verifies source colorimetry, coded geometry, ordered orientation, and optional HDR carriers. `--decode-hdr` reconstructs relative linear BT.2020 pixels from the final file and reports decode precision and RGB bounds. Missing optional metadata is reported as missing. These checks do not establish dynamic display adaptation or device recognition. Dual-JFIF output is available through an explicit JPEG destination. Existing output and default validation retain their current contracts.

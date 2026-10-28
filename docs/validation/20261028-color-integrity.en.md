# ISO rational integrity and alternate ICC transforms

English | [简体中文](20261028-color-integrity.md)

This 2026-10-28 follow-up supersedes the minimum ICC admission boundary in the [earlier audit](20261028-iso-jpeg.en.md). Historical measurements remain unchanged. Test and CI receipts identify the submitted source revision separately. Original fixture bytes and their manifest are not modified.

## Reproduced failures and fixes

Three regressions failed against the original implementation before production edits: a public conversion changed ISO headroom 1/1,000,000 to zero and altered gamma 6001/10 and offset 1/64; a public conversion published an illegal gain interval whose two fractions rounded to the same f64; and alternate ICC admission accepted a CICP-only header without usable transform data. The interval is 2147483646/2147483647 > 2147483645/2147483646, although the decoded doubles agree.

The source ISO path now retains the required rational bytes and compares exact fractions before floating conversion. The HEIF wrapper and the C.4.7 application-space resolution do not alter numeric fields. Supported optional tails are ignored. Valid distinct headrooms that round to the same double remain valid. Edited float observations cannot silently override source fractions. XMP/generated metadata uses checked field-specific rational approximation, with max(1e-12, abs(value) * 1e-9) projection error and explicit failure for unrepresentable values. Every serialized tmap is numerically revalidated. Scene metadata uses the same writer instead of saturating casts. Compatibility XMP is formatted from log-domain values without six/eight-decimal truncation. ISO headroom text retains up to 40 decimal places and uses exact direction comparison, including values that collapse in f64.

Alternate ICC checks now require profile and transform tags, reject partial overlaps, and run bounded f64 CMM transforms with CICP substitution disabled. A correct sRGB transform with CICP BT.2020/PQ fails both standalone admission and public conversion. Complete generated BT.2020 PQ, D65 P3 PQ, nominal HLG and sRGB profiles provide positive transform tests; they are synthetic profiles, not real camera HDR alternate-ICC fixtures or device acceptance evidence. A complete sRGB ICC with matching CICP is copied unchanged into a published tmap, while gain nclx remains independent.

The consistency grid, tolerance, unsupported limited-range and partially unspecified LUT cases, and nominal HLG reference are defined in the [architecture contract](../architecture.en.md#jpeg-gain-map-metadata-selection). This policy may reject a valid profile outside the verified model. It does not rewrite profiles or guess an HDR gamut. The ten-JPEG compatibility corpus contains no alternate ICC, so these added checks do not exclude any of its members. Baseline ICC and XMP-only profile selection remain unchanged.

## Gain precision measurement

The codec regression uses a 93 by 71 smooth RGB ramp, round-to-nearest 8-bit gain codes, the product's quality-90 encoder, padded 512-pixel tile and actual HEVC decoder. It compares decoded codes with the original continuous ramp, including quantization and compression. On Linux x86_64, libheif 1.17.6 and x265 3.5:

| Gain compression | Maximum code error | p99 code error | Maximum EV error at 24 EV | p99 EV error at 24 EV |
| --- | ---: | ---: | ---: | ---: |
| 4:4:4 | 2.796296 | 1.537037 | 0.263181 | 0.144662 |
| 4:2:0 | 4.250000 | 3.130435 | 0.400000 | 0.294629 |

These are observations on one smooth synthetic pattern. The regression ceilings are respectively 4/3 and 6/4 codes for maximum/p99 error; they are not universal quality guarantees. For gamma 1, EV error scales with the gain span. A 24-EV worst-case span therefore magnifies small code errors. The theoretical quantization-only half-step is 0.047059 EV, about 3.32% multiplicative gain error. The actual maximum compression-plus-quantization errors above correspond to larger gain errors. None is a measured whole-image color difference. This patch changes no encoder code point, subsampling mode, bit depth or quality setting.

## Evidence and limits

Reproduce using the configured Rust/libheif HEVC toolchain:

```bash
cargo test --locked -p xdremux-metadata --lib
cargo test --locked -p xdremux-runtime --test iso_jpeg_metadata
cargo test --locked -p xdremux-runtime --lib live_photo::tests
cargo test --locked -p xdremux-codec gain_ramp_quantization -- --nocapture
cargo test --locked -p xdremux-runtime --test live_photo_jpeg_fixtures \
  real_jpeg_motion_photo_vendor_matrix -- --ignored --nocapture
cargo clippy --locked --workspace --all-targets -- --deny warnings
cargo test --locked --workspace --all-targets
(cd fixtures && sha256sum -c SHA256SUMS)
```

The public regressions check actual published tmap bytes, independent base/gain/tmap color properties, input preservation and failure without new pair publication. Exact-HEAD completion and native CI evidence must come from the current receipt/checks, not earlier PR results.

There is still no real camera HDR alternate-ICC positive in the public corpus, no new physical Photos/Android gallery round trip, and no all-stage perceptual error measurement from RGBA32F through final native display. ICC lattice testing cannot prove every LUT value or rendering intent. The current corpus is selected for regression coverage, not representative random sampling. Under an independent representative Bernoulli model, 299 zero-failure cases would give a one-sided 95% lower success bound of at least 99%; neither ten chosen fixtures nor CI check counts satisfy those assumptions. Unsupported-input rejection must be counted separately from successful color-correct conversion.

Sources: [ISO 21496-1:2025](https://www.iso.org/standard/86775.html), 5.2.5.3, 5.2.7 and Annex C; [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf), 9.2.17 and transform/tag requirements; [H.273](https://www.itu.int/rec/T-REC-H.273-202407-I); [moxcms](https://github.com/awxkee/moxcms) (0.9.1). Consistency thresholds, approximation budgets and conservative rejection are XDRemux policies, not quoted ISO or ICC mandates.

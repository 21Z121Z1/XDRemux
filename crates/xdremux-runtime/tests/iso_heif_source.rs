//! A standalone ISO dual-layer HEIF must convert through the standard graph
//! path: its compressed base is preserved, its tone-map payload keeps its bytes,
//! and the reconstructed HDR image survives a profile change.
use std::fs;
use std::path::PathBuf;

use xdremux_engine::{ConversionRequest, HdrExportProfile};
use xdremux_runtime::{analyze_iso_heif, reconstruct_hdr_gain_map, HdrTransfer, PortableRuntime};

fn fixture(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
    .unwrap()
}

fn request(profile: HdrExportProfile) -> ConversionRequest {
    ConversionRequest {
        hdr_profile: profile,
        ..ConversionRequest::default()
    }
}

#[test]
fn an_iso_source_reconverts_to_another_profile_with_the_same_hdr_image() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let first = runtime
        .convert_proxdr_bytes(&source, request(HdrExportProfile::Bt2020Pq), |_| {})
        .expect("ProXDR to PQ conversion must succeed");

    // The first output is a standalone ISO dual-layer HEIF: the standard graph
    // path reads it, not the vendor extension path.
    let prepared = analyze_iso_heif(&first.bytes).expect("output must analyze as an ISO HEIF");
    assert!(!prepared.tmap_payload.is_empty());

    let second = runtime
        .convert_iso_heif_bytes(&first.bytes, request(HdrExportProfile::Bt2020Hlg), |_| {})
        .expect("PQ to HLG conversion must succeed");
    assert_eq!(second.plan.hdr_profile, HdrExportProfile::Bt2020Hlg);
    assert!(
        xdremux_heif::primary_image_signature(&first.bytes).unwrap()
            == xdremux_heif::primary_image_signature(&second.bytes).unwrap(),
        "the compressed SDR base and its colour properties must stay unchanged"
    );
    assert_eq!(
        xdremux_heif::extract_gain_map_hevc_resources(&first.bytes).unwrap(),
        xdremux_heif::extract_gain_map_hevc_resources(&second.bytes).unwrap(),
        "profile changes must keep every compressed Gain Map byte and its precision"
    );
    assert_eq!(
        prepared.tmap_payload,
        analyze_iso_heif(&second.bytes).unwrap().tmap_payload
    );

    // The final file must validate as the selected profile and decode.
    xdremux_heif::validate_hdr_gain_map_profile(&second.bytes, HdrTransfer::Hlg)
        .expect("the published file must declare HLG");

    // Changing the declared encoding must not change the reconstructed image.
    let pq = reconstruct_hdr_gain_map(&first.bytes, HdrTransfer::Pq).expect("PQ reconstruction");
    let hlg =
        reconstruct_hdr_gain_map(&second.bytes, HdrTransfer::Hlg).expect("HLG reconstruction");
    assert_eq!(pq.width, hlg.width);
    assert_eq!(pq.height, hlg.height);
    let mut total = 0.0_f64;
    let mut maximum = 0.0_f64;
    for (a, b) in pq.samples.iter().zip(&hlg.samples) {
        let difference = f64::from((a - b).abs());
        total += difference;
        maximum = maximum.max(difference);
    }
    let mean = total / pq.samples.len() as f64;
    eprintln!("iso reconversion: mean-abs={mean:.6} max-abs={maximum:.6}");
    assert_eq!(
        pq.samples, hlg.samples,
        "a remux must reconstruct identical samples"
    );
}

#[test]
fn the_compatible_layout_is_refused_for_a_non_pq_source() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let hlg = runtime
        .convert_proxdr_bytes(&source, request(HdrExportProfile::Bt2020Hlg), |_| {})
        .expect("ProXDR to HLG conversion must succeed");
    let error = runtime
        .convert_iso_heif_bytes(&hlg.bytes, ConversionRequest::default(), |_| {})
        .expect_err("the compatible layout must not relabel an HLG source");
    assert!(
        error
            .to_string()
            .contains("compatible layout declares BT.2020 PQ"),
        "unexpected error: {error}"
    );
    // The same source converts once a matching profile is selected.
    runtime
        .convert_iso_heif_bytes(&hlg.bytes, request(HdrExportProfile::Bt2020Hlg), |_| {})
        .expect("a matching profile must convert");
}

#[test]
fn jpeg_output_from_an_iso_source_publishes_a_two_image_jpeg() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let pq = runtime
        .convert_proxdr_bytes(&source, request(HdrExportProfile::Bt2020Pq), |_| {})
        .expect("ProXDR to PQ conversion must succeed");
    let jpeg = runtime
        .convert_iso_heif_bytes(
            &pq.bytes,
            ConversionRequest {
                container: xdremux_engine::OutputContainer::Jpeg,
                ..ConversionRequest::default()
            },
            |_| {},
        )
        .expect("an ISO source must convert to a two-image JPEG");
    let index = xdremux_format::dual_jpeg::index_dual_layer_jpeg(&jpeg.bytes)
        .expect("published file must be a two-image JPEG");
    assert!(index.base_has_iso_declaration);
    assert!(index.gain_map_has_iso_parameters);
    assert!(index.base_has_icc, "the Display P3 base must keep its ICC");
    let observed = xdremux_metadata::parse_ultrahdr_gain_map_metadata(
        &jpeg.bytes[index.gain_map.range.clone()],
    )
    .expect("published metadata must parse")
    .expect("published gain map must declare metadata");
    assert!(observed.use_base_color_space);
}

#[test]
fn oppo_compatibility_from_an_iso_source_is_refused() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let pq = runtime
        .convert_proxdr_bytes(&source, request(HdrExportProfile::Bt2020Pq), |_| {})
        .expect("ProXDR to PQ conversion must succeed");
    let error = runtime
        .convert_iso_heif_bytes(
            &pq.bytes,
            ConversionRequest::oppo_gallery_compatible(),
            |_| {},
        )
        .expect_err("OPPO compatibility requires a ProXDR source");
    assert!(
        error
            .to_string()
            .contains("OPPO Gallery compatibility requires a ProXDR source"),
        "unexpected error: {error}"
    );
}

use std::fs;
use std::path::PathBuf;

use xdremux_container::{extract, is_oppo_private_hdr_tail_entry, pack_filtered_oppo_camera_tail};
use xdremux_engine::ConversionRequest;
use xdremux_heif::validate_gain_map_structure;
use xdremux_runtime::PortableRuntime;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn convert(source: &[u8], request: ConversionRequest) -> Vec<u8> {
    PortableRuntime::new()
        .convert_proxdr_bytes(source, request, |_| {})
        .expect("camera-tail conversion should succeed")
        .bytes
}

#[test]
fn oppo_gallery_output_appends_exact_source_tail_bytes() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let extracted = extract(&source).expect("fixture should expose OPPO tail");
    let expected = &source[extracted.manifest_info.extension_start..];
    assert!(!expected.is_empty());

    let output = convert(&source, ConversionRequest::oppo_gallery_compatible());
    validate_gain_map_structure(&output).expect("preserved-tail output must remain valid ISO HDR");
    assert!(output.ends_with(expected));
}

#[test]
fn standard_output_removes_private_hdr_tail_resources() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let extracted = extract(&source).expect("fixture should expose OPPO tail");
    let expected = pack_filtered_oppo_camera_tail(
        &source,
        &extracted.manifest_info,
        extracted.data_base,
        |entry| !is_oppo_private_hdr_tail_entry(&entry.name),
    )
    .expect("expected standard tail should build");

    let output = convert(&source, ConversionRequest::default());
    validate_gain_map_structure(&output).expect("standard output must remain valid ISO HDR");
    assert!(output.ends_with(&expected));
}

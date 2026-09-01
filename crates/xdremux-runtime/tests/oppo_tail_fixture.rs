use std::fs;
use std::path::PathBuf;

use xdremux_container::{
    extract, is_oppo_private_hdr_tail_entry, is_oppo_watermark_tail_entry,
    neutralize_oppo_camera_tail_entries, pack_filtered_oppo_camera_tail,
};
use xdremux_engine::{ConversionRequest, OppoCameraTail, OppoCompatibility};
use xdremux_heif::validate_gain_map_structure;
use xdremux_runtime::PortableRuntime;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn convert(source: &[u8], tail: OppoCameraTail) -> Vec<u8> {
    PortableRuntime::new()
        .convert_proxdr_bytes(
            source,
            ConversionRequest {
                oppo_compatibility: OppoCompatibility::Off,
                oppo_camera_tail: tail,
                ..ConversionRequest::default()
            },
            |_| {},
        )
        .expect("camera-tail conversion should succeed")
        .bytes
}

#[test]
fn preserve_appends_exact_source_tail_bytes() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let extracted = extract(&source).expect("fixture should expose OPPO tail");
    let expected = &source[extracted.manifest_info.extension_start..];
    assert!(!expected.is_empty());

    let output = convert(&source, OppoCameraTail::Preserve);
    validate_gain_map_structure(&output).expect("preserved-tail output must remain valid ISO HDR");
    assert!(output.ends_with(expected));
}

#[test]
fn preserve_no_hdr_matches_swift_name_neutralization_contract() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let extracted = extract(&source).expect("fixture should expose OPPO tail");
    let expected = neutralize_oppo_camera_tail_entries(
        &source,
        &extracted.manifest_info,
        |entry| is_oppo_private_hdr_tail_entry(&entry.name),
    )
    .expect("expected neutralized tail should build");
    assert!(!expected.is_empty());

    let output = convert(&source, OppoCameraTail::PreserveNoHdr);
    validate_gain_map_structure(&output).expect("neutralized-tail output must remain valid ISO HDR");
    assert!(output.ends_with(&expected));
}

#[test]
fn watermark_mode_matches_manifest_filtered_repack() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let extracted = extract(&source).expect("fixture should expose OPPO tail");
    let expected = pack_filtered_oppo_camera_tail(
        &source,
        &extracted.manifest_info,
        extracted.data_base,
        |entry| is_oppo_watermark_tail_entry(&entry.name),
    )
    .expect("expected watermark tail should build");
    assert!(!expected.is_empty(), "fixture should contain watermark metadata");

    let output = convert(&source, OppoCameraTail::Watermark);
    validate_gain_map_structure(&output).expect("watermark-tail output must remain valid ISO HDR");
    assert!(output.ends_with(&expected));
}

use std::fs;
use std::path::PathBuf;

use xdremux_engine::{ConversionRequest, OppoCameraTail, OppoCompatibility};
use xdremux_format::ChromaSampling;
use xdremux_heif::validate_gain_map_structure;
use xdremux_metadata::{
    oppo_tag_flags_in_heif, ISO_ULTRA_HDR_FLAG, LOCAL_HDR_FLAG, OPPO_ULTRA_HDR_FLAG,
};
use xdremux_runtime::PortableRuntime;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn convert(source: &[u8], compatibility: OppoCompatibility) -> Vec<u8> {
    PortableRuntime::new()
        .convert_proxdr_bytes(
            source,
            ConversionRequest {
                oppo_compatibility: compatibility,
                oppo_camera_tail: OppoCameraTail::Off,
                ..ConversionRequest::default()
            },
            |_| {},
        )
        .expect("portable OPPO-compatible conversion should succeed")
        .bytes
}

fn assert_oppo_compatible_rgb420(output: &[u8]) {
    let structure = validate_gain_map_structure(output).expect("output must remain valid ISO HDR");
    assert_eq!(structure.channel_count, 3);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Yuv420);
}

#[test]
fn standard_mode_keeps_x6_gain_map_monochrome() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let output = convert(&source, OppoCompatibility::Off);
    let structure = validate_gain_map_structure(&output).expect("output must remain valid ISO HDR");

    assert_eq!(structure.channel_count, 1);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Mono400);
}

#[test]
fn on_mode_expands_x6_mono_to_rgb420_and_sets_oppo_routing_bit() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let source_flags = oppo_tag_flags_in_heif(&source)
        .expect("source metadata should parse")
        .expect("fixture should contain OPPO routing flags");

    let output = convert(&source, OppoCompatibility::On);
    assert_oppo_compatible_rgb420(&output);
    assert_eq!(
        oppo_tag_flags_in_heif(&output).unwrap(),
        Some(source_flags | OPPO_ULTRA_HDR_FLAG)
    );
}

#[test]
fn iso_no_local_mode_expands_x6_mono_to_rgb420_and_sets_exact_swift_routing_bits() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let source_flags = oppo_tag_flags_in_heif(&source)
        .expect("source metadata should parse")
        .expect("fixture should contain OPPO routing flags");

    let output = convert(&source, OppoCompatibility::IsoNoLocal);
    assert_oppo_compatible_rgb420(&output);
    assert_eq!(
        oppo_tag_flags_in_heif(&output).unwrap(),
        Some((source_flags & !OPPO_ULTRA_HDR_FLAG & !LOCAL_HDR_FLAG) | ISO_ULTRA_HDR_FLAG)
    );
}

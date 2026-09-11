use std::fs;
use std::path::PathBuf;

use xdremux_engine::ConversionRequest;
use xdremux_format::ChromaSampling;
use xdremux_heif::validate_gain_map_structure;
use xdremux_metadata::oppo_tag_flags_in_heif;
use xdremux_runtime::PortableRuntime;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn convert(source: &[u8], request: ConversionRequest) -> Vec<u8> {
    PortableRuntime::new()
        .convert_proxdr_bytes(source, request, |_| {})
        .expect("portable conversion should succeed")
        .bytes
}

#[test]
fn standard_output_keeps_x6_gain_map_monochrome() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let output = convert(&source, ConversionRequest::default());
    let structure = validate_gain_map_structure(&output).expect("output must remain valid ISO HDR");

    assert_eq!(structure.channel_count, 1);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Mono400);
}

#[test]
fn oppo_gallery_output_expands_x6_mono_to_rgb420_and_preserves_vendor_routing() {
    let source = fs::read(fixture()).expect("real OPPO fixture should exist");
    let source_flags = oppo_tag_flags_in_heif(&source)
        .expect("source metadata should parse")
        .expect("fixture should contain OPPO routing flags");

    let output = convert(&source, ConversionRequest::oppo_gallery_compatible());
    let structure = validate_gain_map_structure(&output).expect("output must remain valid ISO HDR");

    assert_eq!(structure.channel_count, 3);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Yuv420);
    assert_eq!(oppo_tag_flags_in_heif(&output).unwrap(), Some(source_flags));
}

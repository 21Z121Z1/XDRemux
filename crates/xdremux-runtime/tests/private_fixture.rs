use std::fs;
use std::path::PathBuf;

use xdremux_engine::ConversionRequest;
use xdremux_format::ChromaSampling;
use xdremux_heif::validate_gain_map_structure;
use xdremux_runtime::PortableRuntime;

#[test]
fn standard_private_proxdr_fixture_reaches_portable_final_file() {
    let Some(root) = std::env::var_os("XDREMUX_REGRESSION_FIXTURE_ROOT") else {
        return;
    };
    let input = PathBuf::from(root).join("standard.heic");
    if !input.is_file() {
        return;
    }

    let source = fs::read(&input).unwrap();
    let receipt = PortableRuntime::new()
        .convert_proxdr_bytes(&source, ConversionRequest::default(), |_| {})
        .unwrap();
    let structure = validate_gain_map_structure(&receipt.bytes).unwrap();

    assert_eq!(structure.channel_count, 3);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Yuv444);
    assert_eq!(structure.luma_bit_depth, 8);
    assert_eq!(structure.chroma_bit_depth, 8);
    assert!(!receipt.bytes.is_empty());
}

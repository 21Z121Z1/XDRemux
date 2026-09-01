use std::fs;
use std::path::PathBuf;

use jpeg_encoder::{ColorType, Encoder};
use xdremux_engine::{ConversionRequest, GainMapChannels, OppoCameraTail};
use xdremux_format::ChromaSampling;
use xdremux_heif::validate_gain_map_structure;
use xdremux_runtime::PortableRuntime;

const JXRS_MARKER: &[u8] = b"\0jxrs";

fn tiny_rgb_jpeg() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(8 * 8 * 3);
    for y in 0_u8..8 {
        for x in 0_u8..8 {
            pixels.extend_from_slice(&[
                x.saturating_mul(32),
                y.saturating_mul(32),
                x.saturating_add(y).saturating_mul(16),
            ]);
        }
    }

    let mut jpeg = Vec::new();
    Encoder::new(&mut jpeg, 100)
        .encode(&pixels, 8, 8, ColorType::Rgb)
        .expect("synthetic JPEG encoding must succeed");
    jpeg
}

fn canonical_uhdr_info() -> [f32; 20] {
    [
        1.0, 1.0, 1.0, 1.0, 4.926, 4.926, 4.926, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        4.926, 4.926, 0.0,
    ]
}

fn synthetic_uhdr_source(base: &[u8]) -> Vec<u8> {
    let mut source = base.to_vec();

    let info_start = source.len();
    for value in canonical_uhdr_info() {
        source.extend_from_slice(&value.to_le_bytes());
    }

    let jpeg = tiny_rgb_jpeg();
    let data_start = source.len();
    source.extend_from_slice(&jpeg);

    let json_start = source.len();
    let info_offset = json_start - info_start;
    let data_offset = json_start - data_start;
    let manifest = format!(
        "[{{\"name\":\"local.uhdr.gainmap.info\",\"offset\":{info_offset},\"length\":80}},{{\"name\":\"local.uhdr.gainmap.data\",\"offset\":{data_offset},\"length\":{}}}]",
        jpeg.len()
    );
    source.extend_from_slice(manifest.as_bytes());
    source.extend_from_slice(JXRS_MARKER);
    let footer_length = u32::try_from(manifest.len() + JXRS_MARKER.len() + 4)
        .expect("synthetic footer length must fit u32");
    source.extend_from_slice(&footer_length.to_le_bytes());
    source
}

fn public_heif_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/20260312_135609..heic")
}

#[test]
fn portable_runtime_converts_synthetic_uhdr_end_to_end() {
    let base = fs::read(public_heif_fixture()).expect("versioned HEIF fixture must be readable");
    let source = synthetic_uhdr_source(&base);
    let runtime = PortableRuntime::new();
    let request = ConversionRequest {
        oppo_camera_tail: OppoCameraTail::Off,
        ..ConversionRequest::default()
    };

    let receipt = runtime
        .convert_proxdr_bytes(&source, request, |_| {})
        .expect("Rust-only synthetic UHDR conversion must succeed");

    assert_eq!(receipt.plan.gain_map_target.channels, GainMapChannels::Rgb);
    assert_eq!(
        receipt.plan.gain_map_target.layout.chroma,
        ChromaSampling::Yuv444
    );
    assert!(!receipt
        .bytes
        .windows(b"local.uhdr.gainmap".len())
        .any(|window| window == b"local.uhdr.gainmap"));
    assert!(!receipt
        .bytes
        .windows(JXRS_MARKER.len())
        .any(|window| window == JXRS_MARKER));

    let structure =
        validate_gain_map_structure(&receipt.bytes).expect("final Rust HEIF graph must validate");
    assert_eq!(structure.channel_count, 3);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Yuv444);
    assert_eq!(structure.luma_bit_depth, 8);
    assert_eq!(structure.chroma_bit_depth, 8);
    assert_eq!(structure.width, 8);
    assert_eq!(structure.height, 8);
}

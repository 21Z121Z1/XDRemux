//! The JPEG output container must publish a two-image JPEG whose carriers and
//! pixels match the source operation. The source is a public ProXDR fixture.
use std::fs;
use std::path::PathBuf;

use xdremux_codec::{
    JpegRasterDecodeRequest, LibHeifProvider, RasterPixelFormat, ZuneJpegProvider,
};
use xdremux_engine::{ConversionRequest, HdrExportProfile, OutputContainer, RasterDecoder};
use xdremux_format::dual_jpeg::index_dual_layer_jpeg;
use xdremux_metadata::parse_ultrahdr_gain_map_metadata;
use xdremux_runtime::PortableRuntime;

fn fixture(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
    .unwrap()
}

fn jpeg_request() -> ConversionRequest {
    ConversionRequest {
        container: OutputContainer::Jpeg,
        ..ConversionRequest::default()
    }
}

#[test]
fn jpeg_output_publishes_both_images_with_their_carriers() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let receipt = runtime
        .convert_proxdr_bytes(&source, jpeg_request(), |_| {})
        .expect("JPEG conversion must succeed");
    assert_eq!(receipt.plan.container, OutputContainer::Jpeg);

    let index = index_dual_layer_jpeg(&receipt.bytes).expect("published JPEG must index");
    assert!(index.base_has_iso_declaration);
    assert!(index.gain_map_has_iso_parameters);
    assert!(
        index.base_has_icc,
        "the Display P3 base must keep its ICC profile"
    );
    assert_eq!(index.mp.image_count, 2);

    // Both images must decode from the published bytes.
    let decoder = ZuneJpegProvider::new();
    let base = decoder
        .decode_raster(&JpegRasterDecodeRequest {
            data: receipt.bytes[index.base.range.clone()].to_vec(),
            format: RasterPixelFormat::Rgb8,
        })
        .expect("published base must decode");
    let gain_format = match index.gain_map.frame.component_count() {
        1 => RasterPixelFormat::Mono8,
        3 => RasterPixelFormat::Rgb8,
        count => panic!("unexpected gain-map component count {count}"),
    };
    let gain = decoder
        .decode_raster(&JpegRasterDecodeRequest {
            data: receipt.bytes[index.gain_map.range.clone()].to_vec(),
            format: gain_format,
        })
        .expect("published gain map must decode");

    // The published base must describe the same image as the source base.
    let boundary = xdremux_container::extract(&source)
        .unwrap()
        .manifest_info
        .extension_start;
    let body = &source[..boundary];
    let primary = xdremux_heif::primary_image_item_id(body).unwrap();
    let expected = LibHeifProvider::new()
        .decode_coded_item_normalized(body, primary, 3)
        .expect("source base must decode");
    assert_eq!(base.width, expected.width);
    assert_eq!(base.height, expected.height);
    let mut total = 0.0_f64;
    let mut maximum = 0.0_f64;
    for (index, value) in expected.samples.iter().enumerate() {
        let row = index / (usize::try_from(expected.width).unwrap() * 3);
        let column = index % (usize::try_from(expected.width).unwrap() * 3);
        let expected = f64::from((value.clamp(0.0, 1.0) * 255.0).round());
        let observed = f64::from(base.data[row * base.bytes_per_row + column]);
        let difference = (expected - observed).abs();
        total += difference;
        maximum = maximum.max(difference);
    }
    let mean = total / expected.samples.len() as f64;
    assert!(
        mean < 2.0 && maximum < 24.0,
        "lossy base re-encode drifted: mean {mean:.3}, max {maximum}"
    );

    // The source Exif payload must survive as an APP1 segment.
    let source_tiff = xdremux_format::heif_exif_tiff(body)
        .expect("source Exif must parse")
        .expect("the fixture must carry Exif");
    let exif_segment = index
        .base
        .segments
        .iter()
        .find(|segment| segment.marker == 0xe1)
        .expect("the published base must carry Exif");
    let mut expected_exif = b"Exif\0\0".to_vec();
    expected_exif.extend_from_slice(&source_tiff);
    assert_eq!(&receipt.bytes[exif_segment.payload.clone()], expected_exif);

    // The gain-map image must keep the source sample geometry.
    assert_eq!(gain.width, receipt.plan.gain_map_target.width);
    assert_eq!(gain.height, receipt.plan.gain_map_target.height);

    // The published metadata must describe the same operation.
    let observed = parse_ultrahdr_gain_map_metadata(&receipt.bytes[index.gain_map.range.clone()])
        .expect("published metadata must parse")
        .expect("published gain map must declare metadata");
    assert!(observed.use_base_color_space);
    assert_eq!(
        observed.channel_count,
        receipt
            .plan
            .gain_map_target
            .channels
            .semantic_channel_count()
    );
}

#[test]
fn jpeg_output_rejects_an_explicit_bt2020_profile() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let request = ConversionRequest {
        container: OutputContainer::Jpeg,
        hdr_profile: HdrExportProfile::Bt2020Pq,
        ..ConversionRequest::default()
    };
    let error = runtime
        .convert_proxdr_bytes(&source, request, |_| {})
        .expect_err("JPEG output cannot declare an explicit BT.2020 profile");
    assert!(
        error.to_string().contains("JPEG output"),
        "unexpected error: {error}"
    );
}

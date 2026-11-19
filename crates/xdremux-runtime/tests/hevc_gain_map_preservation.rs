//! Fixed synthetic 10/12-bit HEVC inputs exercise remux even when the host
//! has only an eight-bit encoder. See vectors/README.md for provenance.
use xdremux_codec::LibHeifProvider;
use xdremux_engine::{ConversionRequest, HdrExportProfile};
use xdremux_heif::extract_gain_map_hevc_resources;
use xdremux_runtime::PortableRuntime;

#[test]
fn remux_keeps_real_high_precision_gain_maps_and_every_compressed_byte() {
    let codec = LibHeifProvider::new();
    for (depth, rgb, source) in [
        (
            10_u8,
            false,
            include_bytes!("vectors/hevc-10-mono.heic").as_slice(),
        ),
        (
            12,
            false,
            include_bytes!("vectors/hevc-12-mono.heic").as_slice(),
        ),
        (
            10,
            true,
            include_bytes!("vectors/hevc-10-rgb.heic").as_slice(),
        ),
        (
            12,
            true,
            include_bytes!("vectors/hevc-12-rgb.heic").as_slice(),
        ),
    ] {
        let first = xdremux_heif::validate_gain_map_structure(source).unwrap();
        assert_eq!(first.luma_bit_depth, depth);
        assert_eq!(first.channel_count, if rgb { 3 } else { 1 });
        let before = codec
            .decode_coded_item_normalized(source, first.gain_map_item_id, if rgb { 3 } else { 1 })
            .unwrap();
        // Low bits are present in the actual decoded source; an 8-bit reduction
        // would change these samples even if the output were relabeled 12-bit.
        assert!(before
            .samples
            .iter()
            .any(|v| (*v * 255.0 - (*v * 255.0).round()).abs() > 0.01));
        let result = PortableRuntime::new()
            .convert_iso_heif_bytes(
                source,
                ConversionRequest {
                    hdr_profile: HdrExportProfile::Bt2020Hlg,
                    ..ConversionRequest::default()
                },
                |_| {},
            )
            .unwrap();
        assert_eq!(result.plan.gain_map_target.layout.luma_bit_depth, depth);
        let final_structure = xdremux_heif::validate_gain_map_structure(&result.bytes).unwrap();
        let after = codec
            .decode_coded_item_normalized(
                &result.bytes,
                final_structure.gain_map_item_id,
                if rgb { 3 } else { 1 },
            )
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(
            xdremux_heif::primary_image_signature(source).unwrap(),
            xdremux_heif::primary_image_signature(&result.bytes).unwrap()
        );
        assert_eq!(
            extract_gain_map_hevc_resources(source).unwrap(),
            extract_gain_map_hevc_resources(&result.bytes).unwrap()
        );
        assert_eq!(
            xdremux_heif::derived_tmap_payload(source).unwrap(),
            xdremux_heif::derived_tmap_payload(&result.bytes).unwrap()
        );
        let jpeg = PortableRuntime::new()
            .convert_iso_heif_bytes(
                source,
                ConversionRequest {
                    container: xdremux_engine::OutputContainer::Jpeg,
                    ..ConversionRequest::default()
                },
                |_| {},
            )
            .unwrap();
        let indexed = xdremux_format::dual_jpeg::index_dual_layer_jpeg(&jpeg.bytes).unwrap();
        assert_eq!(indexed.gain_map.frame.precision, 8);
        let reconstructed = xdremux_runtime::reconstruct_dual_layer_jpeg_hdr(&jpeg.bytes).unwrap();
        assert_eq!((reconstructed.width, reconstructed.height), (64, 64));
    }
}

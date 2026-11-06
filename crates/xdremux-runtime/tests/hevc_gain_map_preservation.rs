//! Generate real 10/12-bit HEVC, remux it through the public runtime, then
//! compare compressed resources and independently decoded samples.
use libheif_rs::{
    Channel, ColorSpace, CompressionFormat, EncoderParameterValue, EncoderQuality, HeifContext,
    Image, LibHeif, RgbChroma,
};
use xdremux_codec::{LibHeifProvider, PrimaryHeifEncodeRequest, Raster8, RasterPixelFormat};
use xdremux_engine::{ConversionRequest, HdrExportProfile};
use xdremux_format::ChromaSampling;
use xdremux_heif::{
    assemble_iso_gain_map_heif, extract_gain_map_hevc_resources, DirectHevcGainMap,
    GainMapChannels, GainMapEncodeProfile, GainMapTile, HdrGainMapProfile, IsoGainMapAssembly,
};
use xdremux_metadata::HdrTransfer;
use xdremux_runtime::PortableRuntime;

#[test]
fn remux_keeps_real_high_precision_gain_maps_and_every_compressed_byte() {
    let codec = LibHeifProvider::new();
    let base = codec
        .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(
            Raster8::new(
                64,
                64,
                64 * 3,
                RasterPixelFormat::Rgb8,
                vec![128; 64 * 64 * 3],
            )
            .unwrap(),
            None,
        ))
        .unwrap();
    let info = [
        1.0, 1.0, 1.0, 1.0, 4.0, 4.0, 4.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 4.0,
    ];
    let tmap = xdremux_metadata::make_apple_tmap_payload(&info).unwrap();
    let xmp = xdremux_metadata::make_hdrgm_xmp(&info).unwrap();
    for (depth, rgb) in [(10_u8, false), (12, false), (10, true), (12, true)] {
        let mut image = Image::new(
            64,
            64,
            if rgb {
                ColorSpace::Rgb(RgbChroma::C444)
            } else {
                ColorSpace::Monochrome
            },
        )
        .unwrap();
        for (component, channel) in if rgb {
            vec![Channel::R, Channel::G, Channel::B]
        } else {
            vec![Channel::Y]
        }
        .into_iter()
        .enumerate()
        {
            image.create_plane(channel, 64, 64, depth).unwrap();
            let planes = image.planes_mut();
            let plane = match channel {
                Channel::R => planes.r,
                Channel::G => planes.g,
                Channel::B => planes.b,
                _ => planes.y,
            }
            .unwrap();
            for y in 0..64 {
                for x in 0..64 {
                    let value = ((x * 37 + y * 19 + component * 137 + 1) % (1 << depth)) as u16;
                    let offset = y * plane.stride + 2 * x;
                    plane.data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        let lib = LibHeif::new_checked().unwrap();
        let mut encoder = lib.encoder_for_format(CompressionFormat::Hevc).unwrap();
        encoder.set_quality(EncoderQuality::LossLess).unwrap();
        if rgb {
            encoder
                .set_parameter_value("chroma", EncoderParameterValue::String("444".into()))
                .unwrap();
        }
        let mut context = HeifContext::new().unwrap();
        context.encode_image(&image, &mut encoder, None).unwrap();
        let encoded = context.write_to_bytes().unwrap();
        let resource = codec.extract_primary_hevc_resource(&encoded).unwrap();
        let tiles = [GainMapTile {
            payload: &resource.payload,
            width: resource.width,
            height: resource.height,
        }];
        let source = assemble_iso_gain_map_heif(
            &base,
            &IsoGainMapAssembly {
                gain_map: DirectHevcGainMap {
                    gain_map_width: 64,
                    gain_map_height: 64,
                    tile_width: 64,
                    tile_height: 64,
                    tiles: &tiles,
                    hvcc: &resource.hvcc,
                    profile: GainMapEncodeProfile {
                        channels: if rgb {
                            GainMapChannels::Rgb
                        } else {
                            GainMapChannels::Mono
                        },
                        chroma: if rgb {
                            ChromaSampling::Yuv444
                        } else {
                            ChromaSampling::Mono400
                        },
                        luma_bit_depth: depth,
                        chroma_bit_depth: depth,
                    },
                },
                tmap_payload: &tmap,
                xmp_payload: &xmp,
                alternate_icc: None,
                profile: Some(HdrGainMapProfile {
                    transfer: HdrTransfer::Pq,
                }),
                hdr_metadata: None,
            },
        )
        .unwrap();
        let first = xdremux_heif::validate_gain_map_structure(&source).unwrap();
        assert_eq!(first.luma_bit_depth, depth);
        let before = codec
            .decode_coded_item_normalized(&source, first.gain_map_item_id, if rgb { 3 } else { 1 })
            .unwrap();
        // Low bits are present in the actual decoded source; an 8-bit reduction
        // would change these samples even if the output were relabeled 12-bit.
        assert!(before
            .samples
            .iter()
            .any(|v| (*v * 255.0 - (*v * 255.0).round()).abs() > 0.01));
        let result = PortableRuntime::new()
            .convert_iso_heif_bytes(
                &source,
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
            extract_gain_map_hevc_resources(&source).unwrap(),
            extract_gain_map_hevc_resources(&result.bytes).unwrap()
        );
        assert_eq!(
            xdremux_heif::derived_tmap_payload(&source).unwrap(),
            xdremux_heif::derived_tmap_payload(&result.bytes).unwrap()
        );
        let jpeg = PortableRuntime::new()
            .convert_iso_heif_bytes(
                &source,
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

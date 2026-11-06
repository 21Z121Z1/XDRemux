//! The opt-in registered APP8 carriers must appear in the published JPEG with
//! offsets that reach the real image boundary, and the enhanced payload must
//! match the source-declared registered metadata.
use std::fs;
use std::path::{Path, PathBuf};

use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    ConversionRequest, GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile,
    GainMapTileEncoder, OutputContainer,
};
use xdremux_format::dual_jpeg::index_dual_layer_jpeg;
use xdremux_format::ChromaSampling;
use xdremux_heif::{
    assemble_iso_gain_map_heif, DirectHevcGainMap, GainMapTile, HdrGainMapProfile,
    HdrMetadataAssembly, IsoGainMapAssembly,
};
use xdremux_metadata::{
    registered_jpeg_base_payload_len, ContentLightLevel, DynamicHdrMetadata, HdrStaticMetadata,
    HdrTransfer, MasteringDisplay, RegisteredHdrMetadata, RegisteredJpegBaseDeclaration,
};
use xdremux_runtime::{validate_dual_layer_jpeg, PortableRuntime};

const APPLE_TMAP_HEX: &str = concat!(
    "000000000040000000000000000100933a9300400000000000000000000100",
    "933a9300400000000000010000000100000000000000010000000000000001"
);
const HDRGM_XMP: &[u8] = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/">
      <hdrgm:Version>1.0</hdrgm:Version>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

fn fixture(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
    .unwrap()
}

fn mastering() -> MasteringDisplay {
    MasteringDisplay {
        primaries: [[8500, 39850], [6550, 2300], [35400, 14600]],
        white_point: [15635, 16450],
        maximum_luminance: 10_000_000,
        minimum_luminance: 50,
    }
}

fn content_light() -> ContentLightLevel {
    ContentLightLevel {
        maximum_content_light_level: 1000,
        maximum_frame_average_light_level: 400,
    }
}

fn expected_registered() -> RegisteredHdrMetadata {
    RegisteredHdrMetadata {
        static_metadata: Some((mastering(), content_light())),
        dynamic_metadata: Some(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap()),
        mapping_extension: None,
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("non-hex byte in test vector"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn gain_map_raster(width: u32, height: u32) -> Raster8 {
    let row_bytes = usize::try_from(width).unwrap() * 3;
    let mut data = vec![0u8; row_bytes * usize::try_from(height).unwrap()];
    for y in 0..height {
        for x in 0..width {
            let value = ((x * 7 + y * 11) & 0xff) as u8;
            let offset = usize::try_from(y).unwrap() * row_bytes + usize::try_from(x).unwrap() * 3;
            data[offset] = value;
            data[offset + 1] = value.wrapping_add(23);
            data[offset + 2] = value.wrapping_add(47);
        }
    }
    Raster8::new(width, height, row_bytes, RasterPixelFormat::Rgb8, data).unwrap()
}

/// Author a canonical ISO dual-layer HEIF that declares registered metadata.
fn iso_source_with_registered_metadata() -> Vec<u8> {
    iso_source_with_metadata(expected_registered())
}

fn iso_source_with_metadata(registered: RegisteredHdrMetadata) -> Vec<u8> {
    // The vendor body has identity orientation, so the two-image JPEG path
    // accepts it. The vendor extension block is dropped before assembly.
    let vendor = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let boundary = xdremux_container::extract(&vendor)
        .unwrap()
        .manifest_info
        .extension_start;
    let base = vendor[..boundary].to_vec();
    let encoded = LibHeifProvider::new()
        .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
            gain_map_raster(16, 12),
            GainMapEncodeProfile {
                width: 16,
                height: 12,
                channels: GainMapChannels::Rgb,
                layout: GainMapCodecLayout {
                    chroma: ChromaSampling::Yuv444,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
        ))
        .unwrap();
    let tiles = encoded
        .tiles
        .iter()
        .map(|tile| GainMapTile {
            payload: &tile.payload,
            width: tile.width,
            height: tile.height,
        })
        .collect::<Vec<_>>();
    let static_metadata = HdrStaticMetadata {
        mastering_display: Some(mastering()),
        content_light_level: Some(content_light()),
    };
    assemble_iso_gain_map_heif(
        &base,
        &IsoGainMapAssembly {
            gain_map: DirectHevcGainMap {
                gain_map_width: encoded.gain_map_width,
                gain_map_height: encoded.gain_map_height,
                tile_width: encoded.tile_width,
                tile_height: encoded.tile_height,
                tiles: &tiles,
                hvcc: &encoded.hvcc,
                profile: xdremux_heif::GainMapEncodeProfile {
                    channels: xdremux_heif::GainMapChannels::Rgb,
                    chroma: ChromaSampling::Yuv444,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
            tmap_payload: &decode_hex(APPLE_TMAP_HEX),
            xmp_payload: HDRGM_XMP,
            alternate_icc: None,
            profile: Some(HdrGainMapProfile {
                transfer: HdrTransfer::Pq,
            }),
            hdr_metadata: Some(HdrMetadataAssembly {
                static_metadata: &static_metadata,
                registered_metadata: Some(&registered),
            }),
        },
    )
    .unwrap()
}

fn jpeg_request(registered: bool) -> ConversionRequest {
    ConversionRequest {
        container: OutputContainer::Jpeg,
        registered_jpeg_metadata: registered,
        ..ConversionRequest::default()
    }
}

#[test]
fn pixel_reconstruction_never_silently_ignores_an_enabled_registered_mapping() {
    use xdremux_metadata::RegisteredMappingExtension;
    let mut section = vec![32];
    for value in [1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        .into_iter()
        .chain([0.5; 8])
    {
        section.extend_from_slice(&value.to_be_bytes());
    }
    for combined in [false, true] {
        let mut extension = Vec::new();
        for raw in if combined {
            [&[][..], &section]
        } else {
            [&section, &[][..]]
        } {
            extension.extend_from_slice(&(raw.len() as u16).to_be_bytes());
            extension.extend_from_slice(raw);
        }
        let mut registered = expected_registered();
        registered.mapping_extension = Some(RegisteredMappingExtension::parse(&extension).unwrap());
        let source = iso_source_with_metadata(registered);
        let error = match xdremux_runtime::reconstruct_hdr_gain_map(&source, HdrTransfer::Pq) {
            Ok(_) => panic!("an enabled unresolved mapping must stop reconstruction"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("registered mapping"), "{error}");
        let work = tempfile::tempdir().unwrap();
        let destination = work.path().join("existing.jpg");
        fs::write(&destination, b"previous output").unwrap();
        assert!(PortableRuntime::new()
            .convert_iso_heif_file(&source, &destination, jpeg_request(false), |_| {})
            .is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"previous output");
        assert!(
            PortableRuntime::new()
                .convert_iso_heif_bytes(&source, jpeg_request(false), |_| {})
                .is_err(),
            "JPEG export must not drop an enabled mapping"
        );
        let jpeg = PortableRuntime::new()
            .convert_iso_heif_bytes(&source, jpeg_request(true), |_| {})
            .unwrap();
        let report = validate_dual_layer_jpeg(Path::new("mapped.jpg"), &jpeg.bytes, false).unwrap();
        let registered = report.registered_metadata.unwrap();
        assert_eq!(
            if combined {
                registered.combined_mapping
            } else {
                registered.base_mapping
            },
            "parsed-not-applied"
        );
        assert_eq!(registered.display_adaptation, "not-applied");
        assert!(validate_dual_layer_jpeg(Path::new("mapped.jpg"), &jpeg.bytes, true).is_err());
        assert!(xdremux_runtime::reconstruct_dual_layer_jpeg_hdr(&jpeg.bytes).is_err());
    }
}

#[test]
fn a_registered_declaration_reaches_the_published_jpeg_with_consistent_offsets() {
    let source = iso_source_with_registered_metadata();
    let runtime = PortableRuntime::new();
    let receipt = runtime
        .convert_iso_heif_bytes(&source, jpeg_request(true), |_| {})
        .expect("registered JPEG conversion must succeed");
    assert!(receipt.plan.registered_jpeg_metadata);

    let index = index_dual_layer_jpeg(&receipt.bytes).expect("published file must index");
    let offsets = index
        .registered_offsets
        .expect("the published file must declare registered offsets");
    assert!(offsets.raw > offsets.reduced);
    assert_eq!(
        index
            .registered_app8_start
            .map(|start| start + offsets.raw as usize),
        Some(index.base.range.end),
        "the raw offset must reach the second Start Of Image"
    );

    // The declaration bytes must repeat the offsets the file satisfies.
    let start = index.registered_app8_start.unwrap();
    let declaration_segment = index
        .base
        .segments
        .iter()
        .find(|segment| segment.marker == 0xe8)
        .expect("base image must carry the declaration");
    let payload = &receipt.bytes[declaration_segment.payload.clone()];
    assert_eq!(payload.len(), registered_jpeg_base_payload_len(2));
    let declaration = RegisteredJpegBaseDeclaration::parse_app8_payload(payload)
        .expect("the published declaration must parse");
    assert_eq!(declaration.image_count, 2);
    assert_eq!(
        declaration.offsets,
        vec![(offsets.raw, offsets.reduced)],
        "declared offsets must match the published layout"
    );
    assert_eq!(start, declaration_segment.range.start);

    // The enhanced image must carry the source registered metadata.
    assert!(index.gain_map_has_registered_payload);
    let enhanced = index
        .gain_map
        .segments
        .iter()
        .find(|segment| segment.marker == 0xe8)
        .expect("gain-map image must carry the enhanced payload");
    let observed =
        RegisteredHdrMetadata::parse_app8_payload(&receipt.bytes[enhanced.payload.clone()])
            .expect("the enhanced payload must parse");
    assert_eq!(observed, expected_registered());
}

#[test]
fn validation_rejects_registered_offsets_that_disagree_with_the_file() {
    let source = iso_source_with_registered_metadata();
    let receipt = PortableRuntime::new()
        .convert_iso_heif_bytes(&source, jpeg_request(true), |_| {})
        .unwrap();
    let index = index_dual_layer_jpeg(&receipt.bytes).unwrap();
    let app8 = index
        .base
        .segments
        .iter()
        .find(|segment| {
            segment.marker == 0xe8 && receipt.bytes[segment.payload.clone()].starts_with(b"ITUT35")
        })
        .unwrap();
    let mut modified = receipt.bytes.clone();
    // The first serialized raw offset starts after the 16-byte header.
    let offset_end = app8.payload.start + 20;
    modified[offset_end - 1] ^= 1;
    // A corrupted APP8 declaration does not invalidate MPF or ISO APP2.
    assert!(index_dual_layer_jpeg(&modified).is_ok());
    let error = validate_dual_layer_jpeg(Path::new("bad-offset.jpg"), &modified, false)
        .expect_err("validation must inspect the serialized APP8 offsets");
    assert!(
        error.to_string().contains("offsets do not match"),
        "{error}"
    );
}

#[test]
fn validation_rejects_corrupt_enhanced_registered_metadata() {
    let source = iso_source_with_registered_metadata();
    let receipt = PortableRuntime::new()
        .convert_iso_heif_bytes(&source, jpeg_request(true), |_| {})
        .unwrap();
    let index = index_dual_layer_jpeg(&receipt.bytes).unwrap();
    let app8 = index
        .gain_map
        .segments
        .iter()
        .find(|segment| {
            segment.marker == 0xe8 && receipt.bytes[segment.payload.clone()].starts_with(b"ITUT35")
        })
        .unwrap();
    let mut modified = receipt.bytes.clone();
    modified[app8.payload.start + 6] ^= 1;
    assert!(index_dual_layer_jpeg(&modified).is_ok());
    let error = validate_dual_layer_jpeg(Path::new("bad-enhanced.jpg"), &modified, false)
        .expect_err("validation must parse the enhanced APP8 payload");
    assert!(
        error
            .to_string()
            .contains("registered JPEG enhanced metadata"),
        "{error}"
    );
}

#[test]
fn a_source_without_registered_metadata_refuses_an_incomplete_registered_pair() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let error = runtime
        .convert_proxdr_bytes(&source, jpeg_request(true), |_| {})
        .expect_err("a registered JPEG needs an enhanced APP8 record");
    assert!(
        error
            .to_string()
            .contains("the source has no registered enhanced metadata to preserve"),
        "{error}"
    );
}

#[test]
fn the_default_jpeg_output_keeps_no_registered_carrier() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let receipt = runtime
        .convert_proxdr_bytes(&source, jpeg_request(false), |_| {})
        .expect("default JPEG conversion must succeed");
    let index = index_dual_layer_jpeg(&receipt.bytes).unwrap();
    assert!(index.registered_offsets.is_none());
    assert!(!index.gain_map_has_registered_payload);
    // The ISO carriers are unchanged by the opt-in.
    assert!(index.base_has_iso_declaration);
    assert!(index.gain_map_has_iso_parameters);
}

#[test]
fn registered_metadata_requires_the_jpeg_container() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let error = runtime
        .convert_proxdr_bytes(
            &source,
            ConversionRequest {
                registered_jpeg_metadata: true,
                ..ConversionRequest::default()
            },
            |_| {},
        )
        .expect_err("registered metadata applies to the JPEG container");
    assert!(
        error
            .to_string()
            .contains("registered APP8 metadata applies to the two-image JPEG container"),
        "unexpected error: {error}"
    );
}

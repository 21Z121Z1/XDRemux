//! Source-declared HDR metadata must survive a real conversion and be read back
//! from the published bytes. The source used here is a public HEIF fixture
//! whose canonical graph is authored once with fixed metadata carriers, then
//! given the vendor extension layout the ProXDR analyzer expects. The metadata
//! values are fixed inputs; no test value is derived from the reader under test.
use std::fs;
use std::path::PathBuf;

use jpeg_encoder::{ColorType, Encoder};
use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    ConversionRequest, GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile,
    GainMapTileEncoder, HdrExportProfile, HdrMetadataOrigin,
};
use xdremux_format::ChromaSampling;
use xdremux_heif::{
    assemble_iso_gain_map_heif, DirectHevcGainMap, GainMapTile, HdrGainMapProfile,
    HdrMetadataAssembly, IsoGainMapAssembly,
};
use xdremux_metadata::{
    ContentLightLevel, DynamicHdrMetadata, HdrStaticMetadata, HdrTransfer, MasteringDisplay,
    RegisteredHdrMetadata,
};
use xdremux_runtime::PortableRuntime;

const JXRS_MARKER: &[u8] = b"\0jxrs";
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
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    assert_eq!(bytes.len() % 2, 0);
    bytes
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

fn tiny_rgb_jpeg() -> Vec<u8> {
    const WIDTH: u8 = 16;
    const HEIGHT: u8 = 12;
    let mut pixels = Vec::with_capacity(usize::from(WIDTH) * usize::from(HEIGHT) * 3);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            pixels.extend_from_slice(&[
                x.saturating_mul(16),
                y.saturating_mul(16),
                x.saturating_add(y).saturating_mul(8),
            ]);
        }
    }
    let mut jpeg = Vec::new();
    Encoder::new(&mut jpeg, 100)
        .encode(&pixels, u16::from(WIDTH), u16::from(HEIGHT), ColorType::Rgb)
        .expect("synthetic JPEG encoding must succeed");
    jpeg
}

/// Author a canonical dual-layer graph carrying fixed HDR metadata carriers.
fn canonical_body_with_metadata(base: &[u8]) -> Vec<u8> {
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
    let registered = expected_registered();
    assemble_iso_gain_map_heif(
        base,
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

/// Wrap a HEIF body in the vendor extension layout the ProXDR analyzer reads.
fn with_vendor_extension(mut body: Vec<u8>) -> Vec<u8> {
    let info = [
        1.0_f32, 1.0, 1.0, 1.0, 4.926, 4.926, 4.926, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        1.0, 4.926, 4.926, 0.0,
    ];
    let info_start = body.len();
    for value in info {
        body.extend_from_slice(&value.to_le_bytes());
    }
    let jpeg = tiny_rgb_jpeg();
    let data_start = body.len();
    body.extend_from_slice(&jpeg);
    let json_start = body.len();
    let info_offset = json_start - info_start;
    let data_offset = json_start - data_start;
    let manifest = format!(
        "[{{\"name\":\"local.uhdr.gainmap.info\",\"offset\":{info_offset},\"length\":80}},{{\"name\":\"local.uhdr.gainmap.data\",\"offset\":{data_offset},\"length\":{}}}]",
        jpeg.len()
    );
    body.extend_from_slice(manifest.as_bytes());
    body.extend_from_slice(JXRS_MARKER);
    let footer_length = u32::try_from(manifest.len() + JXRS_MARKER.len() + 4).unwrap();
    body.extend_from_slice(&footer_length.to_le_bytes());
    body
}

fn metadata_bearing_source() -> Vec<u8> {
    let base = fixture("motion-photo/samsung/heif-ultrahdr-01.heic");
    with_vendor_extension(canonical_body_with_metadata(&base))
}

fn it35_item_count(bytes: &[u8]) -> usize {
    use xdremux_format::isobmff::{parse_meta_box, scan_top_level_boxes, META};
    use xdremux_format::FourCC;
    let top = scan_top_level_boxes(bytes).unwrap();
    let meta = parse_meta_box(bytes, top.boxes.iter().find(|b| b.kind == META).unwrap()).unwrap();
    meta.iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(FourCC::new(*b"it35")))
        .count()
}

#[test]
fn source_declared_hdr_metadata_reaches_the_published_file() {
    let source = metadata_bearing_source();
    let runtime = PortableRuntime::new();
    let request = ConversionRequest {
        hdr_profile: HdrExportProfile::Bt2020Pq,
        ..ConversionRequest::default()
    };
    let receipt = runtime
        .convert_proxdr_bytes(&source, request, |_| {})
        .expect("conversion of a metadata-bearing source must succeed");

    assert_eq!(receipt.plan.hdr_metadata.origin, HdrMetadataOrigin::Source);
    assert!(receipt.plan.hdr_metadata.carries_dynamic_metadata());

    // The published bytes must declare exactly the source carriers, once each.
    let observed = xdremux_heif::extract_hdr_metadata(&receipt.bytes)
        .expect("published HDR metadata must be readable");
    assert_eq!(
        observed.static_metadata,
        HdrStaticMetadata {
            mastering_display: Some(mastering()),
            content_light_level: Some(content_light()),
        }
    );
    assert_eq!(observed.registered_metadata, Some(expected_registered()));
    assert_eq!(it35_item_count(&receipt.bytes), 1);

    // The profile check and the decoded reconstruction still pass with carriers.
    let profile = xdremux_heif::validate_hdr_gain_map_profile(&receipt.bytes, HdrTransfer::Pq)
        .expect("published profile must validate");
    assert_eq!(profile.static_metadata, observed.static_metadata);
    assert_eq!(profile.registered_metadata, observed.registered_metadata);
}

#[test]
fn the_compatible_layout_also_keeps_source_carriers() {
    let source = metadata_bearing_source();
    let runtime = PortableRuntime::new();
    let receipt = runtime
        .convert_proxdr_bytes(&source, ConversionRequest::default(), |_| {})
        .expect("compatible conversion of a metadata-bearing source must succeed");

    assert_eq!(receipt.plan.hdr_metadata.origin, HdrMetadataOrigin::Source);
    // The consumer layout has no explicit BT.2020 profile; its carriers still
    // describe the reconstructed image and must not be dropped.
    xdremux_heif::validate_gain_map_structure(&receipt.bytes)
        .expect("compatible layout must stay structurally valid");
    let observed = xdremux_heif::extract_hdr_metadata(&receipt.bytes).unwrap();
    assert_eq!(
        observed.static_metadata,
        HdrStaticMetadata {
            mastering_display: Some(mastering()),
            content_light_level: Some(content_light()),
        }
    );
    assert_eq!(observed.registered_metadata, Some(expected_registered()));
}

#[test]
fn a_source_without_hdr_metadata_gains_none() {
    let source = fixture("proxdr/oppo/find-x9-ultra/uhdr-master-v1-01.heic");
    let runtime = PortableRuntime::new();
    let request = ConversionRequest {
        hdr_profile: HdrExportProfile::Bt2020Pq,
        ..ConversionRequest::default()
    };
    let receipt = runtime
        .convert_proxdr_bytes(&source, request, |_| {})
        .expect("conversion of a real fixture must succeed");

    assert_eq!(receipt.plan.hdr_metadata.origin, HdrMetadataOrigin::Absent);
    assert!(receipt.plan.hdr_metadata.is_empty());
    let observed = xdremux_heif::extract_hdr_metadata(&receipt.bytes).unwrap();
    assert!(observed.is_empty());
    assert_eq!(it35_item_count(&receipt.bytes), 0);
}

#[test]
fn a_conflicting_source_does_not_replace_an_existing_output() {
    let mut source = metadata_bearing_source();
    let position = source
        .windows(4)
        .position(|window| window == b"mdcv")
        .expect("the authored source declares a mastering display");
    source[position + 4 + 16 + 3] ^= 0x40;

    static UNIQUE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let work = std::env::temp_dir().join(format!(
        "xdremux-hdr-metadata-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        UNIQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&work).unwrap();
    let output = work.join("output.heic");
    fs::write(&output, b"previous-output").unwrap();

    let runtime = PortableRuntime::new();
    let request = ConversionRequest {
        hdr_profile: HdrExportProfile::Bt2020Pq,
        ..ConversionRequest::default()
    };
    assert!(runtime
        .convert_proxdr_file(&source, &output, request, |_| {})
        .is_err());
    assert_eq!(fs::read(&output).unwrap(), b"previous-output");
    fs::remove_dir_all(work).ok();
}

#[test]
fn conflicting_static_carriers_fail_before_publication() {
    let mut source = metadata_bearing_source();
    let position = source
        .windows(4)
        .position(|window| window == b"mdcv")
        .expect("the authored source declares a mastering display");
    // Maximum luminance is the first of the two trailing words of the 24-byte
    // payload. Change it so the registered record disagrees with the property.
    source[position + 4 + 16 + 3] ^= 0x40;
    let error = xdremux_runtime::analyze_proxdr(&source).unwrap_err();
    assert!(error.to_string().contains("mastering display"), "{error}");
}

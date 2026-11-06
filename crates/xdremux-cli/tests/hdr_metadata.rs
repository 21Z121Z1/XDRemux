//! The convert entry point must report the HDR metadata it preserved and the
//! published file must declare exactly those carriers.
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jpeg_encoder::{ColorType, Encoder};
use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile, GainMapTileEncoder,
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

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xdremux-cli-hdr-metadata-{}-{stamp}",
        std::process::id()
    ))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
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

fn tiny_rgb_jpeg() -> Vec<u8> {
    // 8x7 matches the fixture primary's coded 8:7 aspect.
    const WIDTH: u8 = 8;
    const HEIGHT: u8 = 7;
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

/// Build a ProXDR-layout source whose derived HDR image declares fixed
/// metadata carriers.
fn metadata_bearing_source() -> Vec<u8> {
    let fixture_bytes = fixture("proxdr/oppo/find-x9-ultra/uhdr-master-v1-01.heic");
    let boundary = xdremux_container::extract(&fixture_bytes)
        .unwrap()
        .manifest_info
        .extension_start;
    let base = &fixture_bytes[..boundary];
    let encoded = LibHeifProvider::new()
        .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
            gain_map_raster(8, 7),
            GainMapEncodeProfile {
                width: 8,
                height: 7,
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
    let registered = RegisteredHdrMetadata {
        static_metadata: Some((mastering(), content_light())),
        dynamic_metadata: Some(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap()),
        mapping_extension: None,
    };
    let mut body = assemble_iso_gain_map_heif(
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
    .unwrap();

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
    let manifest = format!(
        "[{{\"name\":\"local.uhdr.gainmap.info\",\"offset\":{},\"length\":80}},{{\"name\":\"local.uhdr.gainmap.data\",\"offset\":{},\"length\":{}}}]",
        json_start - info_start,
        json_start - data_start,
        jpeg.len()
    );
    body.extend_from_slice(manifest.as_bytes());
    body.extend_from_slice(JXRS_MARKER);
    let footer = u32::try_from(manifest.len() + JXRS_MARKER.len() + 4).unwrap();
    body.extend_from_slice(&footer.to_le_bytes());
    body
}

#[test]
fn convert_reports_preserved_hdr_metadata_and_publishes_it() {
    let work = unique_dir();
    fs::create_dir_all(&work).expect("temporary test directory must be created");
    let input = work.join("source.heic");
    let output = work.join("output.heic");
    fs::write(&input, metadata_bearing_source()).unwrap();

    let args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        path_arg(&input),
        OsString::from("--output"),
        path_arg(&output),
        OsString::from("--hdr-profile"),
        OsString::from("bt2020-pq"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    assert_eq!(
        code,
        0,
        "CLI conversion failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    let stdout = String::from_utf8(stdout).expect("CLI stdout must be UTF-8");
    assert!(stdout.contains("converted:"), "{stdout}");
    assert!(
        stdout.contains(
            "HDR metadata preserved from source: mastering display, content light level, dynamic metadata"
        ),
        "{stdout}"
    );

    let converted = fs::read(&output).expect("CLI must publish the output file");
    let observed = xdremux_heif::extract_hdr_metadata(&converted)
        .expect("published HDR metadata must be readable");
    assert_eq!(
        observed.static_metadata.mastering_display,
        Some(mastering())
    );
    assert_eq!(
        observed.static_metadata.content_light_level,
        Some(content_light())
    );
    assert!(observed.has_dynamic_metadata());
    fs::remove_dir_all(work).ok();
}

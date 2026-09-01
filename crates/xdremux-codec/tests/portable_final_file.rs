use std::fs;
use std::path::PathBuf;

use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    GainMapChannels as EngineChannels, GainMapCodecLayout, GainMapEncodeProfile as EngineProfile,
    GainMapTileEncoder,
};
use xdremux_format::ChromaSampling;
use xdremux_heif::{
    assemble_iso_gain_map_heif, validate_gain_map_structure, DirectHevcGainMap,
    GainMapChannels as HeifChannels, GainMapEncodeProfile as HeifProfile, GainMapTile,
    IsoGainMapAssembly,
};

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

fn source_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/20260312_135609..heic")
}

fn rgb_raster(width: u32, height: u32) -> Raster8 {
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

fn mono_raster(width: u32, height: u32) -> Raster8 {
    let row_bytes = usize::try_from(width).unwrap();
    let mut data = vec![0u8; row_bytes * usize::try_from(height).unwrap()];
    for y in 0..height {
        for x in 0..width {
            data[usize::try_from(y).unwrap() * row_bytes + usize::try_from(x).unwrap()] =
                ((x * 5 + y * 13) & 0xff) as u8;
        }
    }
    Raster8::new(width, height, row_bytes, RasterPixelFormat::Mono8, data).unwrap()
}

fn engine_profile(
    width: u32,
    height: u32,
    channels: EngineChannels,
    chroma: ChromaSampling,
) -> EngineProfile {
    EngineProfile {
        width,
        height,
        channels,
        layout: GainMapCodecLayout {
            chroma,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        },
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("non-hex byte in test vector"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn assemble_and_validate(
    source: &[u8],
    encoded: &xdremux_codec::EncodedGainMapTiles,
) -> xdremux_heif::GainMapStructure {
    let tiles: Vec<GainMapTile<'_>> = encoded
        .tiles
        .iter()
        .map(|tile| GainMapTile {
            payload: &tile.payload,
            width: tile.width,
            height: tile.height,
        })
        .collect();
    let channels = match encoded.profile.channels {
        EngineChannels::Mono => HeifChannels::Mono,
        EngineChannels::Rgb => HeifChannels::Rgb,
    };
    let tmap = decode_hex(APPLE_TMAP_HEX);
    assert_eq!(tmap.len(), 62);
    let output = assemble_iso_gain_map_heif(
        source,
        &IsoGainMapAssembly {
            gain_map: DirectHevcGainMap {
                gain_map_width: encoded.gain_map_width,
                gain_map_height: encoded.gain_map_height,
                tile_width: encoded.tile_width,
                tile_height: encoded.tile_height,
                tiles: &tiles,
                hvcc: &encoded.hvcc,
                profile: HeifProfile {
                    channels,
                    chroma: encoded.profile.layout.chroma,
                    luma_bit_depth: encoded.profile.layout.luma_bit_depth,
                    chroma_bit_depth: encoded.profile.layout.chroma_bit_depth,
                },
            },
            tmap_payload: &tmap,
            xmp_payload: HDRGM_XMP,
        },
    )
    .expect("assemble libheif tiles directly into final HEIF");
    validate_gain_map_structure(&output).expect("validate final HEIF structure")
}

#[test]
fn portable_libheif_profiles_produce_valid_final_gain_map_files_without_swift_intermediate() {
    let source = fs::read(source_fixture()).expect("read public HEIF fixture");
    let provider = LibHeifProvider::new();
    let width = 641;
    let height = 513;

    let cases = [
        (
            rgb_raster(width, height),
            EngineChannels::Rgb,
            ChromaSampling::Yuv420,
        ),
        (
            rgb_raster(width, height),
            EngineChannels::Rgb,
            ChromaSampling::Yuv444,
        ),
        (
            mono_raster(width, height),
            EngineChannels::Mono,
            ChromaSampling::Mono400,
        ),
    ];

    for (raster, channels, chroma) in cases {
        let request = GainMapTileEncodeRequest::reference_compatible(
            raster,
            engine_profile(width, height, channels, chroma),
        );
        let encoded = provider
            .encode_gain_map_tiles(&request)
            .unwrap_or_else(|error| panic!("libheif {chroma:?} encode failed: {error}"));
        let structure = assemble_and_validate(&source, &encoded);
        assert_eq!(structure.width, width);
        assert_eq!(structure.height, height);
        assert_eq!(structure.channel_count, channels.semantic_channel_count());
        assert_eq!(structure.chroma_sampling, chroma);
        assert_eq!(structure.luma_bit_depth, 8);
        assert_eq!(structure.chroma_bit_depth, 8);
        assert_eq!(structure.rows, 2);
        assert_eq!(structure.columns, 2);
    }
}

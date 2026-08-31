use std::fs;
use std::path::PathBuf;

use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    GainMapChannels as EngineChannels, GainMapCodecLayout, GainMapEncodeProfile as EngineProfile,
    GainMapTileEncoder,
};
use xdremux_format::ChromaSampling;
use xdremux_heif::{
    replace_private_jpeg_gain_map_with_hevc_tiles, validate_gain_map_structure, DirectHevcGainMap,
    GainMapChannels as HeifChannels, GainMapEncodeProfile as HeifProfile, GainMapTile,
};

fn source_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/20260312_135609..heic")
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
    let output = replace_private_jpeg_gain_map_with_hevc_tiles(
        source,
        &DirectHevcGainMap {
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
    )
    .expect("assemble libheif tiles into final HEIF");
    validate_gain_map_structure(&output).expect("validate final HEIF structure")
}

#[test]
fn portable_libheif_profiles_produce_valid_final_gain_map_files() {
    let source = fs::read(source_fixture()).expect("read public ProXDR fixture");
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

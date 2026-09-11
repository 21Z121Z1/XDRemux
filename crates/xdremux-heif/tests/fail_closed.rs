use xdremux_format::ChromaSampling;
use xdremux_heif::{
    replace_private_jpeg_gain_map_with_hevc_tiles, DirectHevcGainMap, GainMapChannels,
    GainMapEncodeProfile, GainMapTile,
};

fn hvcc() -> Vec<u8> {
    let mut value = vec![0u8; 19];
    value[0] = 1;
    value[1] = 4;
    value[16] = 3;
    value
}

#[test]
fn malformed_source_fails_closed_after_validating_spec() {
    let payload = [0x01u8];
    let tile = GainMapTile {
        payload: &payload,
        width: 4,
        height: 4,
    };
    let hvcc = hvcc();
    let spec = DirectHevcGainMap {
        gain_map_width: 4,
        gain_map_height: 4,
        tile_width: 4,
        tile_height: 4,
        tiles: &[tile],
        hvcc: &hvcc,
        profile: GainMapEncodeProfile {
            channels: GainMapChannels::Rgb,
            chroma: ChromaSampling::Yuv444,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        },
    };

    // Declares a largesize box but truncates the 16-byte header. The hardened
    // format layer must return an error instead of indexing or panicking.
    let malformed = [0, 0, 0, 1, b'f', b't', b'y', b'p', 0, 0, 0, 0];
    assert!(replace_private_jpeg_gain_map_with_hevc_tiles(&malformed, &spec).is_err());
}

#[test]
fn structurally_incomplete_source_is_rejected() {
    let payload = [0x01u8];
    let tile = GainMapTile {
        payload: &payload,
        width: 4,
        height: 4,
    };
    let hvcc = hvcc();
    let spec = DirectHevcGainMap {
        gain_map_width: 4,
        gain_map_height: 4,
        tile_width: 4,
        tile_height: 4,
        tiles: &[tile],
        hvcc: &hvcc,
        profile: GainMapEncodeProfile {
            channels: GainMapChannels::Rgb,
            chroma: ChromaSampling::Yuv444,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        },
    };

    let ftyp_only = [0, 0, 0, 8, b'f', b't', b'y', b'p'];
    assert!(replace_private_jpeg_gain_map_with_hevc_tiles(&ftyp_only, &spec).is_err());
}

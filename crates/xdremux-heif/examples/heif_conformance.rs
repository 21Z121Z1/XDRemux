use std::env;
use std::fs;
use std::path::PathBuf;

use xdremux_heif::{
    replace_private_jpeg_gain_map_with_hevc_tiles, validate_gain_map_structure,
    DirectHevcGainMap, GainMapTile,
};

fn parse_u32(value: &str, name: &str) -> u32 {
    value
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}: {value}"))
}

fn parse_u8(value: &str, name: &str) -> u8 {
    value
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}: {value}"))
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(
        value.len().is_multiple_of(2),
        "hex input must have even length"
    );
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty(), "hex input must have even length");
    pairs
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex must be ASCII");
            u8::from_str_radix(text, 16).unwrap_or_else(|_| panic!("invalid hex byte: {text}"))
        })
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() >= 9, "usage: heif_conformance SOURCE OUTPUT WIDTH HEIGHT TILE_WIDTH TILE_HEIGHT CHANNELS HVCC_HEX [TILE_HEX:WIDTH:HEIGHT ...]");

    let source_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);
    let gain_map_width = parse_u32(&args[3], "gain-map width");
    let gain_map_height = parse_u32(&args[4], "gain-map height");
    let tile_width = parse_u32(&args[5], "tile width");
    let tile_height = parse_u32(&args[6], "tile height");
    let channel_count = parse_u8(&args[7], "channel count");
    let hvcc = decode_hex(&args[8]);

    let tile_storage: Vec<(Vec<u8>, u32, u32)> = args[9..]
        .iter()
        .map(|arg| {
            let mut parts = arg.split(':');
            let payload = decode_hex(parts.next().expect("tile payload missing"));
            let width = parse_u32(parts.next().expect("tile width missing"), "tile width");
            let height = parse_u32(parts.next().expect("tile height missing"), "tile height");
            assert!(
                parts.next().is_none(),
                "tile spec has too many fields: {arg}"
            );
            (payload, width, height)
        })
        .collect();
    let tiles: Vec<GainMapTile<'_>> = tile_storage
        .iter()
        .map(|(payload, width, height)| GainMapTile {
            payload,
            width: *width,
            height: *height,
        })
        .collect();

    let source = fs::read(&source_path).expect("read source HEIF");
    let output = replace_private_jpeg_gain_map_with_hevc_tiles(
        &source,
        &DirectHevcGainMap {
            gain_map_width,
            gain_map_height,
            tile_width,
            tile_height,
            tiles: &tiles,
            hvcc: &hvcc,
            channel_count,
        },
    )
    .unwrap_or_else(|error| panic!("Rust HEIF writer failed: {error}"));
    let validated = validate_gain_map_structure(&output)
        .unwrap_or_else(|error| panic!("Rust HEIF structural validation failed: {error}"));
    assert_eq!(validated.width, gain_map_width, "validated Gain Map width");
    assert_eq!(validated.height, gain_map_height, "validated Gain Map height");
    assert_eq!(
        validated.channel_count, channel_count,
        "validated Gain Map channel count"
    );
    assert_eq!(
        validated.tile_item_ids.len(),
        tiles.len(),
        "validated Gain Map tile count"
    );
    fs::write(output_path, output).expect("write Rust HEIF output");
}

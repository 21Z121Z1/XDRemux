//! Rotated HEIF gain-map round trip on the public Samsung fixture.
//!
//! The primary item is stored 4000x3000 with `irot` 3. A gain map built from
//! that coded luma must keep the coded aspect, inherit `irot` 3, and line up
//! with the base after libheif applies transforms. The tone-map item is the
//! display size with `irot` 0.

use std::fs;
use std::path::PathBuf;

use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    GainMapChannels as EngineChannels, GainMapCodecLayout, GainMapEncodeProfile as EngineProfile,
    GainMapTileEncoder, RasterDecoder,
};
use xdremux_format::isobmff::{
    parse_boxes, parse_irot_quarter_turns, parse_ispe_dimensions, parse_meta_box,
    scan_top_level_boxes, META,
};
use xdremux_format::{ChromaSampling, FourCC};
use xdremux_heif::{
    assemble_iso_gain_map_heif, primary_coded_geometry, validate_gain_map_structure,
    DirectHevcGainMap, GainMapChannels as HeifChannels, GainMapEncodeProfile as HeifProfile,
    GainMapTile, IsoGainMapAssembly,
};

const ISPE: FourCC = FourCC::new(*b"ispe");
const IROT: FourCC = FourCC::new(*b"irot");
const GAIN_WIDTH: u32 = 80;
const GAIN_HEIGHT: u32 = 60;

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic")
}

fn decode_hex(value: &str) -> Vec<u8> {
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
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

fn box_downsample_luma(raster: &Raster8, dst_width: u32, dst_height: u32) -> Raster8 {
    let src_width = usize::try_from(raster.width).unwrap();
    let src_height = usize::try_from(raster.height).unwrap();
    let width = usize::try_from(dst_width).unwrap();
    let height = usize::try_from(dst_height).unwrap();
    let mut sum = vec![0_u64; width * height];
    let mut count = vec![0_u32; width * height];
    for y in 0..src_height {
        let destination_y = y * height / src_height;
        for x in 0..src_width {
            let destination_x = x * width / src_width;
            let offset = y * raster.bytes_per_row + x * 3;
            let luma = u64::from(raster.data[offset])
                + u64::from(raster.data[offset + 1])
                + u64::from(raster.data[offset + 2]);
            let bin = destination_y * width + destination_x;
            sum[bin] += luma;
            count[bin] += 1;
        }
    }
    let mut data = Vec::with_capacity(width * height * 3);
    for (total, samples) in sum.iter().zip(&count) {
        let luma = u8::try_from(total / u64::from(*samples) / 3).unwrap();
        data.extend_from_slice(&[luma, luma, luma]);
    }
    Raster8::new(
        dst_width,
        dst_height,
        width * 3,
        RasterPixelFormat::Rgb8,
        data,
    )
    .unwrap()
}

fn assemble(
    source: &[u8],
    encoded: &xdremux_codec::EncodedGainMapTiles,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, xdremux_heif::HeifError> {
    let tiles: Vec<GainMapTile<'_>> = encoded
        .tiles
        .iter()
        .map(|tile| GainMapTile {
            payload: &tile.payload,
            width: tile.width,
            height: tile.height,
        })
        .collect();
    assemble_iso_gain_map_heif(
        source,
        &IsoGainMapAssembly {
            alternate_icc: None,
            gain_map: DirectHevcGainMap {
                gain_map_width: width,
                gain_map_height: height,
                tile_width: encoded.tile_width,
                tile_height: encoded.tile_height,
                tiles: &tiles,
                hvcc: &encoded.hvcc,
                profile: HeifProfile {
                    channels: HeifChannels::Rgb,
                    chroma: ChromaSampling::Yuv444,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
            tmap_payload: &decode_hex(APPLE_TMAP_HEX),
            xmp_payload: HDRGM_XMP,
        },
    )
}

fn associated_ispe_and_irot(source: &[u8], item_id: u32) -> (u32, u32, u8) {
    let top = scan_top_level_boxes(source).unwrap();
    let meta_header = top.boxes.iter().find(|header| header.kind == META).unwrap();
    let meta = parse_meta_box(source, meta_header).unwrap();
    let entry = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .unwrap();
    let mut ispe = None;
    let mut irot = None;
    for association in &entry.associations {
        let property = meta
            .properties
            .iter()
            .find(|property| property.index == u32::from(association.property_index))
            .unwrap();
        let headers = parse_boxes(source, property.box_range.clone()).unwrap();
        let header = headers.first().unwrap();
        if property.kind == ISPE {
            ispe = Some(parse_ispe_dimensions(source, header).unwrap());
        } else if property.kind == IROT {
            irot = Some(parse_irot_quarter_turns(source, header).unwrap());
        }
    }
    let (width, height) = ispe.unwrap();
    (width, height, irot.unwrap())
}

fn luma_at(raster: &Raster8, x: usize, y: usize) -> f64 {
    let bytes_per_pixel = raster.format.bytes_per_pixel();
    let offset = y * raster.bytes_per_row + x * bytes_per_pixel;
    if bytes_per_pixel == 1 {
        f64::from(raster.data[offset])
    } else {
        0.2126 * f64::from(raster.data[offset])
            + 0.7152 * f64::from(raster.data[offset + 1])
            + 0.0722 * f64::from(raster.data[offset + 2])
    }
}

fn box_luma(raster: &Raster8, dst_width: usize, dst_height: usize) -> Vec<f64> {
    let src_width = usize::try_from(raster.width).unwrap();
    let src_height = usize::try_from(raster.height).unwrap();
    let mut sum = vec![0.0; dst_width * dst_height];
    let mut count = vec![0_u32; dst_width * dst_height];
    for y in 0..src_height {
        let destination_y = y * dst_height / src_height;
        for x in 0..src_width {
            let destination_x = x * dst_width / src_width;
            let bin = destination_y * dst_width + destination_x;
            sum[bin] += luma_at(raster, x, y);
            count[bin] += 1;
        }
    }
    sum.iter()
        .zip(&count)
        .map(|(total, samples)| total / f64::from(*samples))
        .collect()
}

fn edges(samples: &[f64], width: usize, height: usize) -> Vec<f64> {
    let mut magnitude = vec![0.0; samples.len()];
    for y in 0..height {
        for x in 0..width {
            let here = samples[y * width + x];
            let horizontal = if x + 1 < width {
                (samples[y * width + x + 1] - here).abs()
            } else {
                0.0
            };
            let vertical = if y + 1 < height {
                (samples[(y + 1) * width + x] - here).abs()
            } else {
                0.0
            };
            magnitude[y * width + x] = horizontal + vertical;
        }
    }
    magnitude
}

fn rotate_90_cw(samples: &[f64], width: usize, height: usize) -> Vec<f64> {
    let mut rotated = vec![0.0; samples.len()];
    for y in 0..height {
        for x in 0..width {
            let destination_x = height - 1 - y;
            let destination_y = x;
            rotated[destination_y * height + destination_x] = samples[y * width + x];
        }
    }
    rotated
}

fn pearson(left: &[f64], right: &[f64]) -> f64 {
    assert_eq!(left.len(), right.len());
    let count = left.len() as f64;
    let mean_left = left.iter().sum::<f64>() / count;
    let mean_right = right.iter().sum::<f64>() / count;
    let mut numerator = 0.0;
    let mut left_energy = 0.0;
    let mut right_energy = 0.0;
    for (lhs, rhs) in left.iter().zip(right) {
        let left_delta = lhs - mean_left;
        let right_delta = rhs - mean_right;
        numerator += left_delta * right_delta;
        left_energy += left_delta * left_delta;
        right_energy += right_delta * right_delta;
    }
    if left_energy == 0.0 || right_energy == 0.0 {
        return 0.0;
    }
    numerator / (left_energy.sqrt() * right_energy.sqrt())
}

fn write_imageio_artifact(
    output: &[u8],
    primary_coded: (u32, u32),
    primary_display: (u32, u32),
    quarter_turns: u8,
    gain_display: (u32, u32),
) {
    let Ok(directory) = std::env::var("XDREMUX_ORIENTATION_OUTPUT") else {
        return;
    };
    let directory = PathBuf::from(directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("oriented-gain-map.heic"), output).unwrap();
    let expected = format!(
        concat!(
            "{{\n",
            "  \"primary_coded_width\": {coded_w},\n",
            "  \"primary_coded_height\": {coded_h},\n",
            "  \"primary_display_width\": {display_w},\n",
            "  \"primary_display_height\": {display_h},\n",
            "  \"quarter_turns_ccw\": {turns},\n",
            "  \"gain_coded_width\": {gain_w},\n",
            "  \"gain_coded_height\": {gain_h},\n",
            "  \"gain_display_width\": {gain_display_w},\n",
            "  \"gain_display_height\": {gain_display_h}\n",
            "}}\n"
        ),
        coded_w = primary_coded.0,
        coded_h = primary_coded.1,
        display_w = primary_display.0,
        display_h = primary_display.1,
        turns = quarter_turns,
        gain_w = GAIN_WIDTH,
        gain_h = GAIN_HEIGHT,
        gain_display_w = gain_display.0,
        gain_display_h = gain_display.1,
    );
    fs::write(directory.join("expected.json"), expected).unwrap();
}

#[test]
fn rotated_gain_map_matches_primary_coded_orientation_after_libheif_transform() {
    let source = fs::read(source_fixture()).expect("public Samsung HEIF fixture");
    let geometry = primary_coded_geometry(&source).unwrap();
    assert_eq!((geometry.width, geometry.height), (4000, 3000));
    assert_eq!(geometry.quarter_turns_ccw, 3);

    let provider = LibHeifProvider::new();
    let display = provider
        .decode_raster(&xdremux_codec::HeifRasterDecodeRequest {
            data: source.clone(),
            format: RasterPixelFormat::Rgb8,
        })
        .expect("decode oriented primary");
    assert_eq!((display.width, display.height), (3000, 4000));
    let coded = display.undo_irot(geometry.quarter_turns_ccw).unwrap();
    assert_eq!((coded.width, coded.height), (4000, 3000));
    let gain = box_downsample_luma(&coded, GAIN_WIDTH, GAIN_HEIGHT);

    let encoded = provider
        .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
            gain,
            EngineProfile {
                width: GAIN_WIDTH,
                height: GAIN_HEIGHT,
                channels: EngineChannels::Rgb,
                layout: GainMapCodecLayout {
                    chroma: ChromaSampling::Yuv444,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
        ))
        .expect("encode coded gain map");

    let rejected = assemble(&source, &encoded, GAIN_HEIGHT, GAIN_WIDTH).unwrap_err();
    let rejected = rejected.to_string();
    assert!(
        rejected.contains("Gain Map") && rejected.contains("coded orientation"),
        "{rejected}"
    );

    let output = assemble(&source, &encoded, GAIN_WIDTH, GAIN_HEIGHT).unwrap();
    let structure = validate_gain_map_structure(&output).unwrap();
    assert_eq!(
        (structure.width, structure.height),
        (GAIN_WIDTH, GAIN_HEIGHT)
    );

    let (gain_width, gain_height, gain_irot) =
        associated_ispe_and_irot(&output, structure.gain_map_item_id);
    assert_eq!(
        (gain_width, gain_height, gain_irot),
        (GAIN_WIDTH, GAIN_HEIGHT, 3)
    );
    let (tmap_width, tmap_height, tmap_irot) =
        associated_ispe_and_irot(&output, structure.tmap_item_id);
    assert_eq!((tmap_width, tmap_height, tmap_irot), (3000, 4000, 0));

    let decoded_primary = provider
        .decode_raster(&xdremux_codec::HeifRasterDecodeRequest {
            data: output.clone(),
            format: RasterPixelFormat::Rgb8,
        })
        .unwrap();
    let decoded_gain = provider
        .decode_item_raster(&output, structure.gain_map_item_id, RasterPixelFormat::Rgb8)
        .unwrap();
    assert_eq!((decoded_gain.width, decoded_gain.height), (60, 80));
    write_imageio_artifact(
        &output,
        (geometry.width, geometry.height),
        (decoded_primary.width, decoded_primary.height),
        geometry.quarter_turns_ccw,
        (decoded_gain.width, decoded_gain.height),
    );

    let width = usize::try_from(decoded_gain.width).unwrap();
    let height = usize::try_from(decoded_gain.height).unwrap();
    let base = box_luma(&decoded_primary, width, height);
    let aligned_gain = box_luma(&decoded_gain, width, height);
    let aligned_edges = pearson(
        &edges(&base, width, height),
        &edges(&aligned_gain, width, height),
    );
    let turned = rotate_90_cw(&aligned_gain, width, height);
    let turned_base = box_luma(&decoded_primary, height, width);
    let turned_edges = pearson(
        &edges(&turned_base, height, width),
        &edges(&turned, height, width),
    );
    let value_correlation = pearson(&base, &aligned_gain);
    eprintln!(
        "orientation-roundtrip value={value_correlation:.4} edges={aligned_edges:.4} rotated_edges={turned_edges:.4}"
    );
    assert!(
        value_correlation > 0.85,
        "aligned luma correlation {value_correlation}"
    );
    assert!(
        aligned_edges > 0.5 && aligned_edges > turned_edges + 0.25,
        "aligned edge correlation {aligned_edges}, rotated {turned_edges}"
    );
}

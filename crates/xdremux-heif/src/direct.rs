use std::collections::{HashMap, HashSet};

use xdremux_format::isobmff::{
    make_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box, make_iref_box,
    make_ispe_box, parse_boxes, parse_iinf, parse_iloc, parse_ipco_properties, parse_ipma,
    parse_iref, parse_pitm, scan_top_level_boxes, BoxHeader, IlocEntry, IlocExtent,
    IpmaAssociation, IpmaEntry, IrefEntry, ItemInfo, PropertyInfo, FTYP, IDAT, IINF, ILOC,
    IPMA, IPRP, IREF, MDAT, META, PITM,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const GRID: FourCC = FourCC::new(*b"grid");
const JPEG: FourCC = FourCC::new(*b"jpeg");
const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const TMAP: FourCC = FourCC::new(*b"tmap");
const MIME: FourCC = FourCC::new(*b"mime");
const EXIF: FourCC = FourCC::new(*b"Exif");
const IPCO: FourCC = FourCC::new(*b"ipco");
const ISPE: FourCC = FourCC::new(*b"ispe");
const IROT: FourCC = FourCC::new(*b"irot");
const COLR: FourCC = FourCC::new(*b"colr");
const PIXI: FourCC = FourCC::new(*b"pixi");
const DIMG: FourCC = FourCC::new(*b"dimg");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const GRPL: FourCC = FourCC::new(*b"grpl");
const ALTR: FourCC = FourCC::new(*b"altr");

// Exact property bytes used by the current Swift writer. Keeping these local to
// the writer avoids broadening the already-sealed xdremux-format contract.
const COLR_SRGB_BOX: &[u8] = &[
    0x00, 0x00, 0x00, 0x13, 0x63, 0x6f, 0x6c, 0x72, 0x6e, 0x63, 0x6c, 0x78, 0x00, 0x02,
    0x00, 0x02, 0x00, 0x02, 0x80,
];
const COLR_UNSPECIFIED_BT601_BOX: &[u8] = &[
    0x00, 0x00, 0x00, 0x13, 0x63, 0x6f, 0x6c, 0x72, 0x6e, 0x63, 0x6c, 0x78, 0x00, 0x02,
    0x00, 0x02, 0x00, 0x06, 0x80,
];
const PIXI_MONO8_BOX: &[u8] = &[
    0x00, 0x00, 0x00, 0x0e, 0x70, 0x69, 0x78, 0x69, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08,
];
const PIXI_RGB8_BOX: &[u8] = &[
    0x00, 0x00, 0x00, 0x10, 0x70, 0x69, 0x78, 0x69, 0x00, 0x00, 0x00, 0x00, 0x03, 0x08,
    0x08, 0x08,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainMapTile<'a> {
    pub payload: &'a [u8],
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectHevcGainMap<'a> {
    pub gain_map_width: u32,
    pub gain_map_height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tiles: &'a [GainMapTile<'a>],
    /// HEVCDecoderConfigurationRecord payload, without the outer hvcC box.
    pub hvcc: &'a [u8],
    pub channel_count: u8,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn checked_range(start: usize, length: usize, limit: usize, context: &str) -> Result<std::ops::Range<usize>> {
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid(format!("{context} range overflows")))?;
    if end > limit {
        return Err(invalid(format!("{context} range exceeds source")));
    }
    Ok(start..end)
}

fn raw_box<'a>(source: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    source
        .get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside source")))
}

fn raw_property<'a>(source: &'a [u8], property: &PropertyInfo) -> Result<&'a [u8]> {
    source
        .get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside source", property.index)))
}

fn child<'a>(children: &'a [BoxHeader], kind: FourCC, context: &str) -> Result<&'a BoxHeader> {
    children
        .iter()
        .find(|header| header.kind == kind)
        .ok_or_else(|| invalid(format!("direct HEVC Gain Map source {context}/{} missing", kind)))
}

fn item_is(item: &ItemInfo, kind: FourCC) -> bool {
    item.item_type == Some(kind)
}

fn ceil_div(value: u32, divisor: u32, context: &str) -> Result<u32> {
    value
        .checked_add(divisor - 1)
        .map(|sum| sum / divisor)
        .ok_or_else(|| invalid(format!("{context} geometry overflows")))
}

fn validate_spec(spec: &DirectHevcGainMap<'_>) -> Result<(u32, u32)> {
    if spec.gain_map_width == 0
        || spec.gain_map_height == 0
        || spec.tile_width == 0
        || spec.tile_height == 0
        || !matches!(spec.channel_count, 1 | 3)
    {
        return Err(invalid("direct HEVC Gain Map has unsupported tile geometry"));
    }
    let columns = ceil_div(spec.gain_map_width, spec.tile_width, "gain-map columns")?;
    let rows = ceil_div(spec.gain_map_height, spec.tile_height, "gain-map rows")?;
    let expected_count = usize::try_from(
        rows.checked_mul(columns)
            .ok_or_else(|| invalid("direct HEVC Gain Map tile count overflows"))?,
    )
    .map_err(|_| invalid("direct HEVC Gain Map tile count exceeds usize"))?;
    if spec.tiles.len() != expected_count || spec.tiles.is_empty() {
        return Err(invalid(
            "direct HEVC Gain Map tile count does not match its geometry",
        ));
    }
    for row in 0..rows {
        for column in 0..columns {
            let index = usize::try_from(row * columns + column)
                .map_err(|_| invalid("direct HEVC Gain Map tile index exceeds usize"))?;
            let tile = spec.tiles[index];
            if tile.payload.is_empty() {
                return Err(invalid("direct HEVC Gain Map contains an empty tile"));
            }
            let expected_width = spec
                .gain_map_width
                .checked_sub(
                    column
                        .checked_mul(spec.tile_width)
                        .ok_or_else(|| invalid("direct HEVC Gain Map tile x overflows"))?,
                )
                .map(|remaining| remaining.min(spec.tile_width))
                .ok_or_else(|| invalid("direct HEVC Gain Map tile x exceeds image"))?;
            let expected_height = spec
                .gain_map_height
                .checked_sub(
                    row.checked_mul(spec.tile_height)
                        .ok_or_else(|| invalid("direct HEVC Gain Map tile y overflows"))?,
                )
                .map(|remaining| remaining.min(spec.tile_height))
                .ok_or_else(|| invalid("direct HEVC Gain Map tile y exceeds image"))?;
            let logical_edge = tile.width == expected_width && tile.height == expected_height;
            let padded_full = tile.width == spec.tile_width && tile.height == spec.tile_height;
            if !logical_edge && !padded_full {
                return Err(invalid(
                    "direct HEVC Gain Map edge tile geometry is inconsistent",
                ));
            }
        }
    }

    if spec.hvcc.len() <= 18
        || spec.hvcc[0] != 1
        || spec.hvcc[1] & 0x1f != 4
        || spec.hvcc[16] & 0x03 != if spec.channel_count == 1 { 0 } else { 3 }
        || (spec.hvcc[17] & 0x07) + 8 != 8
        || (spec.hvcc[18] & 0x07) + 8 != 8
    {
        return Err(invalid(
            "direct HEVC Gain Map codec does not match its channel layout",
        ));
    }
    Ok((rows, columns))
}

fn make_grid_payload(rows: u32, columns: u32, width: u32, height: u32) -> Result<Vec<u8>> {
    if !(1..=256).contains(&rows) || !(1..=256).contains(&columns) || width == 0 || height == 0 {
        return Err(invalid("invalid HEIF grid geometry"));
    }
    let large = width > u32::from(u16::MAX) || height > u32::from(u16::MAX);
    let mut payload = vec![0, u8::from(large), (rows - 1) as u8, (columns - 1) as u8];
    if large {
        payload.extend_from_slice(&width.to_be_bytes());
        payload.extend_from_slice(&height.to_be_bytes());
    } else {
        payload.extend_from_slice(&(width as u16).to_be_bytes());
        payload.extend_from_slice(&(height as u16).to_be_bytes());
    }
    Ok(payload)
}

fn make_imageio_tmap_ispe(primary_ispe: &[u8], irot: &[u8]) -> Result<Vec<u8>> {
    if primary_ispe.len() < 20 || &primary_ispe[4..8] != b"ispe" || irot.len() < 9 || &irot[4..8] != b"irot" {
        return Err(invalid("cannot derive oriented tmap geometry"));
    }
    let width = u32::from_be_bytes(primary_ispe[12..16].try_into().expect("checked length"));
    let height = u32::from_be_bytes(primary_ispe[16..20].try_into().expect("checked length"));
    let quarter_turns = irot[8] & 0x03;
    if quarter_turns.is_multiple_of(2) {
        Ok(make_ispe_box(width, height)?)
    } else {
        Ok(make_ispe_box(height, width)?)
    }
}

fn next_property(output: &mut Vec<u8>, next_index: &mut u16, raw: &[u8]) -> Result<u16> {
    let index = *next_index;
    if index == 0 {
        return Err(invalid("HEIF property index overflowed"));
    }
    output.extend_from_slice(raw);
    *next_index = next_index
        .checked_add(1)
        .ok_or_else(|| invalid("HEIF property index exceeds u16"))?;
    Ok(index)
}

fn shift_offset(value: u64, delta: i128) -> Result<u64> {
    let shifted = i128::from(value)
        .checked_add(delta)
        .ok_or_else(|| invalid("iloc file offset shift overflows"))?;
    u64::try_from(shifted).map_err(|_| invalid("iloc file offset shift is negative or too large"))
}

fn make_altr_group(group_id: u32, tmap_id: u32, primary_id: u32) -> Result<Vec<u8>> {
    let mut payload = vec![0, 0, 0, 0];
    payload.extend_from_slice(&group_id.to_be_bytes());
    payload.extend_from_slice(&2u32.to_be_bytes());
    payload.extend_from_slice(&tmap_id.to_be_bytes());
    payload.extend_from_slice(&primary_id.to_be_bytes());
    let altr = make_box(ALTR, &payload)?;
    Ok(make_box(GRPL, &altr)?)
}

fn source_brand_order(source: &[u8], ftyp: &BoxHeader) -> Result<Vec<[u8; 4]>> {
    let payload = source
        .get(ftyp.payload_range())
        .ok_or_else(|| invalid("ftyp payload is outside source"))?;
    if payload.len() < 8 {
        return Err(invalid("ftyp payload is shorter than major brand/minor version"));
    }
    let mut brands = Vec::new();
    for chunk in payload[8..].chunks_exact(4) {
        brands.push(chunk.try_into().expect("chunks_exact(4)"));
    }
    Ok(brands)
}

fn build_ftyp(source: &[u8], ftyp: &BoxHeader) -> Result<Vec<u8>> {
    let payload = source
        .get(ftyp.payload_range())
        .ok_or_else(|| invalid("ftyp payload is outside source"))?;
    if payload.len() < 8 {
        return Err(invalid("ftyp payload is shorter than major brand/minor version"));
    }
    let mut output_payload = payload[..8].to_vec();
    let mut ordered: Vec<[u8; 4]> = [*b"mif1", *b"tmap", *b"MiHE", *b"miaf", *b"MiHB", *b"heic"].to_vec();
    for brand in source_brand_order(source, ftyp)? {
        if !ordered.contains(&brand) {
            ordered.push(brand);
        }
    }
    for brand in ordered {
        output_payload.extend_from_slice(&brand);
    }
    Ok(make_box(FTYP, &output_payload)?)
}

fn find_associated_property<'a>(
    item_id: u32,
    kind: FourCC,
    ipma_entries: &'a [IpmaEntry],
    property_by_index: &HashMap<u32, &'a PropertyInfo>,
) -> Option<&'a PropertyInfo> {
    ipma_entries
        .iter()
        .find(|entry| entry.item_id == item_id)?
        .associations
        .iter()
        .filter_map(|association| property_by_index.get(&u32::from(association.property_index)).copied())
        .find(|property| property.kind == kind)
}

fn mapped_associations(
    entry: &IpmaEntry,
    remapped: &HashMap<u32, u16>,
) -> Result<Vec<IpmaAssociation>> {
    entry
        .associations
        .iter()
        .map(|association| {
            let old = u32::from(association.property_index);
            let property_index = remapped
                .get(&old)
                .copied()
                .ok_or_else(|| invalid(format!("direct HEVC Gain Map cannot remap Base property {old}")))?;
            Ok(IpmaAssociation {
                property_index,
                essential: association.essential,
            })
        })
        .collect()
}

fn resolved_extents(entry: &IlocEntry) -> Result<Vec<(u64, u64)>> {
    entry
        .extents
        .iter()
        .map(|extent| {
            Ok((entry.resolved_extent_offset(extent)?, extent.length))
        })
        .collect()
}

/// Replaces the private JPEG Gain Map graph produced by the Swift passthrough
/// stage with an HEVC tiled grid. The encoded tile samples and hvcC record are
/// supplied by the caller; this function performs no image or codec work.
pub fn replace_private_jpeg_gain_map_with_hevc_tiles(
    source: &[u8],
    spec: &DirectHevcGainMap<'_>,
) -> Result<Vec<u8>> {
    let (rows, columns) = validate_spec(spec)?;
    let top = scan_top_level_boxes(source)?;
    let ftyp = top
        .boxes
        .iter()
        .find(|header| header.kind == FTYP)
        .ok_or_else(|| invalid("direct HEVC Gain Map replacement requires ftyp/meta/mdat"))?;
    let meta = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("direct HEVC Gain Map replacement requires ftyp/meta/mdat"))?;
    let mdat = top
        .boxes
        .iter()
        .find(|header| header.kind == MDAT)
        .ok_or_else(|| invalid("direct HEVC Gain Map replacement requires ftyp/meta/mdat"))?;
    let meta_children_start = meta
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    if meta_children_start > meta.data_end {
        return Err(invalid("meta full-box header is truncated"));
    }
    let meta_children = parse_boxes(source, meta_children_start..meta.data_end)?;
    let iinf_header = child(&meta_children, IINF, "meta")?;
    let iloc_header = child(&meta_children, ILOC, "meta")?;
    let pitm_header = child(&meta_children, PITM, "meta")?;
    let iprp_header = child(&meta_children, IPRP, "meta")?;
    let idat_header = child(&meta_children, IDAT, "meta")?;
    let iref_header = child(&meta_children, IREF, "meta")?;

    let primary_id = parse_pitm(source, pitm_header)?;
    let item_info = parse_iinf(source, iinf_header)?;
    let item_by_id: HashMap<u32, &ItemInfo> = item_info
        .entries
        .iter()
        .map(|item| (item.item_id, item))
        .collect();
    let refs = parse_iref(source, iref_header)?;
    let tmap_id = item_info
        .entries
        .iter()
        .find(|item| {
            item_is(item, TMAP)
                && refs.entries.iter().any(|reference| {
                    reference.kind == DIMG
                        && reference.from_item_id == item.item_id
                        && reference.to_item_ids.contains(&primary_id)
                })
        })
        .map(|item| item.item_id)
        .ok_or_else(|| invalid("private JPEG Gain Map graph is missing"))?;
    let gain_map_id = refs
        .entries
        .iter()
        .find(|reference| reference.kind == DIMG && reference.from_item_id == tmap_id)
        .and_then(|reference| {
            reference.to_item_ids.iter().copied().find(|item_id| {
                *item_id != primary_id
                    && item_by_id
                        .get(item_id)
                        .is_some_and(|item| item_is(item, JPEG))
            })
        })
        .ok_or_else(|| invalid("private JPEG Gain Map graph is missing"))?;

    let iloc = parse_iloc(source, iloc_header)?;
    let iloc_by_id: HashMap<u32, &IlocEntry> = iloc
        .entries
        .iter()
        .map(|entry| (entry.item_id, entry))
        .collect();
    let jpeg_entry = iloc_by_id
        .get(&gain_map_id)
        .copied()
        .ok_or_else(|| invalid("private JPEG Gain Map has no iloc entry"))?;
    if jpeg_entry.construction_method != 0 || jpeg_entry.extents.len() != 1 {
        return Err(invalid(
            "private JPEG Gain Map does not have one file extent",
        ));
    }
    let jpeg_extent = &jpeg_entry.extents[0];
    let jpeg_offset = jpeg_entry.resolved_extent_offset(jpeg_extent)?;
    let jpeg_end = jpeg_offset
        .checked_add(jpeg_extent.length)
        .ok_or_else(|| invalid("private JPEG Gain Map extent overflows"))?;
    let mdat_start = u64::try_from(mdat.data_start)
        .map_err(|_| invalid("mdat data offset exceeds u64"))?;
    let mdat_end = u64::try_from(mdat.data_end)
        .map_err(|_| invalid("mdat end offset exceeds u64"))?;
    if jpeg_offset < mdat_start || jpeg_end != mdat_end {
        return Err(invalid(
            "private JPEG Gain Map is not the final mdat payload",
        ));
    }

    let properties = parse_ipco_properties(source, iprp_header)?;
    let iprp_children = parse_boxes(source, iprp_header.payload_range())?;
    let ipma_header = iprp_children
        .iter()
        .find(|header| header.kind == IPMA)
        .ok_or_else(|| invalid("private JPEG Gain Map source ipco/ipma missing"))?;
    let ipma = parse_ipma(source, ipma_header)?;

    let xmp_ids: Vec<u32> = item_info
        .entries
        .iter()
        .filter(|item| {
            item_is(item, MIME)
                && refs.entries.iter().any(|reference| {
                    reference.kind == CDSC
                        && reference.from_item_id == item.item_id
                        && reference.to_item_ids.contains(&tmap_id)
                })
        })
        .map(|item| item.item_id)
        .collect();
    let mut appended_graph_ids: HashSet<u32> = HashSet::from([gain_map_id, tmap_id]);
    appended_graph_ids.extend(xmp_ids.iter().copied());
    let original_maximum_item_id = item_info
        .entries
        .iter()
        .filter(|item| !appended_graph_ids.contains(&item.item_id))
        .map(|item| item.item_id)
        .max()
        .ok_or_else(|| invalid("direct HEVC Gain Map source has no original items"))?;

    let mut tile_ids = Vec::with_capacity(spec.tiles.len());
    for offset in 1..=spec.tiles.len() {
        let offset = u32::try_from(offset)
            .map_err(|_| invalid("direct HEVC Gain Map tile count exceeds u32"))?;
        tile_ids.push(
            original_maximum_item_id
                .checked_add(offset)
                .ok_or_else(|| invalid("direct HEVC Gain Map item ID overflows"))?,
        );
    }
    let output_gain_map_id = original_maximum_item_id
        .checked_add(
            u32::try_from(spec.tiles.len())
                .map_err(|_| invalid("direct HEVC Gain Map tile count exceeds u32"))?,
        )
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| invalid("direct HEVC Gain Map item ID overflows"))?;
    let output_tmap_id = output_gain_map_id
        .checked_add(1)
        .ok_or_else(|| invalid("direct HEVC Gain Map item ID overflows"))?;
    if output_tmap_id >= u32::from(u16::MAX) {
        return Err(invalid("direct HEVC Gain Map requires 16-bit item IDs"));
    }
    let xmp_id = xmp_ids.first().copied();
    let output_item_id = |item_id: u32| -> u32 {
        if item_id == gain_map_id {
            output_gain_map_id
        } else if item_id == tmap_id {
            output_tmap_id
        } else {
            item_id
        }
    };

    let property_by_index: HashMap<u32, &PropertyInfo> = properties
        .iter()
        .map(|property| (property.index, property))
        .collect();
    let original_ipma_entries: Vec<&IpmaEntry> = ipma
        .entries
        .iter()
        .filter(|entry| !appended_graph_ids.contains(&entry.item_id))
        .collect();
    let original_property_indices: HashSet<u32> = original_ipma_entries
        .iter()
        .flat_map(|entry| {
            entry
                .associations
                .iter()
                .map(|association| u32::from(association.property_index))
        })
        .collect();
    let original_properties: Vec<&PropertyInfo> = properties
        .iter()
        .filter(|property| original_property_indices.contains(&property.index))
        .collect();
    let original_non_codec: Vec<&PropertyInfo> = original_properties
        .iter()
        .copied()
        .filter(|property| property.kind != HVCC)
        .collect();
    let original_codec: Vec<&PropertyInfo> = original_properties
        .iter()
        .copied()
        .filter(|property| property.kind == HVCC)
        .collect();
    if original_codec.is_empty() {
        return Err(invalid("direct HEVC Gain Map source Base has no hvcC"));
    }

    let mut ipco_payload = Vec::new();
    let mut remapped_property_indices: HashMap<u32, u16> = HashMap::new();
    let mut next_property_index: u16 = 1;
    for property in &original_non_codec {
        let index = next_property(&mut ipco_payload, &mut next_property_index, raw_property(source, property)?)?;
        remapped_property_indices.insert(property.index, index);
    }
    let gain_colr_index = next_property(
        &mut ipco_payload,
        &mut next_property_index,
        if spec.channel_count == 1 {
            COLR_SRGB_BOX
        } else {
            COLR_UNSPECIFIED_BT601_BOX
        },
    )?;
    let gain_grid_ispe = make_ispe_box(spec.gain_map_width, spec.gain_map_height)?;
    let gain_grid_ispe_index = next_property(
        &mut ipco_payload,
        &mut next_property_index,
        &gain_grid_ispe,
    )?;

    let tmap_color = find_associated_property(tmap_id, COLR, &ipma.entries, &property_by_index)
        .ok_or_else(|| invalid("direct HEVC Gain Map tmap properties are incomplete"))?;
    let tmap_pixi = find_associated_property(tmap_id, PIXI, &ipma.entries, &property_by_index)
        .ok_or_else(|| invalid("direct HEVC Gain Map tmap properties are incomplete"))?;
    let tmap_color_index = next_property(
        &mut ipco_payload,
        &mut next_property_index,
        raw_property(source, tmap_color)?,
    )?;
    let tmap_pixi_index = next_property(
        &mut ipco_payload,
        &mut next_property_index,
        raw_property(source, tmap_pixi)?,
    )?;

    let gain_pixi_index = if spec.channel_count == 1 {
        next_property(&mut ipco_payload, &mut next_property_index, PIXI_MONO8_BOX)?
    } else if let Some(existing) = original_non_codec
        .iter()
        .find(|property| raw_property(source, property).is_ok_and(|raw| raw == PIXI_RGB8_BOX))
    {
        *remapped_property_indices
            .get(&existing.index)
            .ok_or_else(|| invalid("RGB pixi property remap is missing"))?
    } else {
        next_property(&mut ipco_payload, &mut next_property_index, PIXI_RGB8_BOX)?
    };

    let mut unique_tile_sizes: Vec<(u32, u32)> = Vec::new();
    for tile in spec.tiles {
        let size = (tile.width, tile.height);
        if !unique_tile_sizes.contains(&size) {
            unique_tile_sizes.push(size);
        }
    }
    let mut tile_ispe_by_size: HashMap<(u32, u32), u16> = HashMap::new();
    for size in unique_tile_sizes {
        let raw_ispe = make_ispe_box(size.0, size.1)?;
        let index = if let Some(existing) = original_non_codec
            .iter()
            .find(|property| raw_property(source, property).is_ok_and(|raw| raw == raw_ispe))
        {
            *remapped_property_indices
                .get(&existing.index)
                .ok_or_else(|| invalid("tile ispe property remap is missing"))?
        } else {
            next_property(&mut ipco_payload, &mut next_property_index, &raw_ispe)?
        };
        tile_ispe_by_size.insert(size, index);
    }

    let primary_ipma = ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == primary_id)
        .ok_or_else(|| invalid("direct HEVC Gain Map Base has no ipma entry"))?;
    let primary_ispe = primary_ipma
        .associations
        .iter()
        .filter_map(|association| property_by_index.get(&u32::from(association.property_index)).copied())
        .find(|property| property.kind == ISPE)
        .ok_or_else(|| invalid("direct HEVC Gain Map Base properties lack ispe/irot"))?;
    let irot = primary_ipma
        .associations
        .iter()
        .filter_map(|association| property_by_index.get(&u32::from(association.property_index)).copied())
        .find(|property| property.kind == IROT)
        .or_else(|| original_non_codec.iter().copied().find(|property| property.kind == IROT))
        .ok_or_else(|| invalid("direct HEVC Gain Map Base properties lack ispe/irot"))?;
    let primary_ispe_index = *remapped_property_indices
        .get(&primary_ispe.index)
        .ok_or_else(|| invalid("primary ispe property remap is missing"))?;
    let irot_index = *remapped_property_indices
        .get(&irot.index)
        .ok_or_else(|| invalid("irot property remap is missing"))?;
    let tmap_ispe = make_imageio_tmap_ispe(raw_property(source, primary_ispe)?, raw_property(source, irot)?)?;
    let tmap_ispe_index = if raw_property(source, primary_ispe)? == tmap_ispe {
        primary_ispe_index
    } else {
        next_property(&mut ipco_payload, &mut next_property_index, &tmap_ispe)?
    };

    for property in &original_codec {
        let index = next_property(&mut ipco_payload, &mut next_property_index, raw_property(source, property)?)?;
        remapped_property_indices.insert(property.index, index);
    }
    let hvcc_box = make_box(HVCC, spec.hvcc)?;
    let tile_hvcc_index = next_property(
        &mut ipco_payload,
        &mut next_property_index,
        &hvcc_box,
    )?;

    let mut output_ipma_entries = Vec::new();
    for entry in original_ipma_entries {
        output_ipma_entries.push(IpmaEntry {
            item_id: entry.item_id,
            associations: mapped_associations(entry, &remapped_property_indices)?,
        });
    }
    for (tile_id, tile) in tile_ids.iter().copied().zip(spec.tiles.iter().copied()) {
        let tile_ispe = *tile_ispe_by_size
            .get(&(tile.width, tile.height))
            .ok_or_else(|| invalid("direct HEVC Gain Map tile ispe mapping is missing"))?;
        output_ipma_entries.push(IpmaEntry {
            item_id: tile_id,
            associations: vec![
                IpmaAssociation { property_index: tile_ispe, essential: true },
                IpmaAssociation { property_index: gain_colr_index, essential: true },
                IpmaAssociation { property_index: tile_hvcc_index, essential: true },
            ],
        });
    }
    output_ipma_entries.push(IpmaEntry {
        item_id: output_gain_map_id,
        associations: vec![
            IpmaAssociation { property_index: gain_colr_index, essential: true },
            IpmaAssociation { property_index: gain_grid_ispe_index, essential: false },
            IpmaAssociation { property_index: gain_pixi_index, essential: false },
            IpmaAssociation { property_index: irot_index, essential: true },
        ],
    });
    output_ipma_entries.push(IpmaEntry {
        item_id: output_tmap_id,
        associations: vec![
            IpmaAssociation { property_index: tmap_color_index, essential: true },
            IpmaAssociation { property_index: tmap_ispe_index, essential: false },
            IpmaAssociation { property_index: tmap_pixi_index, essential: false },
            IpmaAssociation { property_index: irot_index, essential: true },
        ],
    });
    let ipco_part = make_box(IPCO, &ipco_payload)?;
    let ipma_part = make_ipma_box(ipma.version, ipma.flags, &output_ipma_entries)?;
    let mut iprp_payload = ipco_part;
    iprp_payload.extend_from_slice(&ipma_part);
    let iprp_part = make_box(IPRP, &iprp_payload)?;

    let mut raw_infes = Vec::new();
    for item in &item_info.entries {
        if !appended_graph_ids.contains(&item.item_id) {
            raw_infes.extend_from_slice(
                source
                    .get(item.box_range.clone())
                    .ok_or_else(|| invalid(format!("infe {} is outside source", item.item_id)))?,
            );
        }
    }
    let mut iinf_entries: Vec<Vec<u8>> = Vec::new();
    // Re-split original raw infes because make_iinf_box accepts complete child boxes.
    let original_infe_boxes = parse_boxes(&raw_infes, 0..raw_infes.len())?;
    for header in original_infe_boxes {
        iinf_entries.push(raw_box(&raw_infes, &header, "infe")?.to_vec());
    }
    for tile_id in &tile_ids {
        iinf_entries.push(make_infe_box(*tile_id, HVC1, 1)?);
    }
    iinf_entries.push(make_infe_box(
        output_gain_map_id,
        GRID,
        item_by_id.get(&gain_map_id).map_or(1, |item| item.flags),
    )?);
    iinf_entries.push(make_infe_box(
        output_tmap_id,
        TMAP,
        item_by_id.get(&tmap_id).map_or(0, |item| item.flags),
    )?);
    let iinf_part = make_iinf_box(item_info.version, &iinf_entries)?;

    let tmap_entry = iloc_by_id
        .get(&tmap_id)
        .copied()
        .ok_or_else(|| invalid("private JPEG Gain Map tmap has no iloc entry"))?;
    if tmap_entry.construction_method != 1 || tmap_entry.extents.len() != 1 {
        return Err(invalid(
            "private JPEG Gain Map tmap does not have one idat extent",
        ));
    }
    let old_tmap_extent = &tmap_entry.extents[0];
    let old_tmap_offset = tmap_entry.resolved_extent_offset(old_tmap_extent)?;
    let source_idat_payload = source
        .get(idat_header.payload_range())
        .ok_or_else(|| invalid("private JPEG Gain Map idat payload is outside source"))?;
    let old_tmap_offset_usize = usize::try_from(old_tmap_offset)
        .map_err(|_| invalid("private JPEG Gain Map tmap offset exceeds usize"))?;
    let old_tmap_length_usize = usize::try_from(old_tmap_extent.length)
        .map_err(|_| invalid("private JPEG Gain Map tmap length exceeds usize"))?;
    let old_tmap_range = checked_range(
        old_tmap_offset_usize,
        old_tmap_length_usize,
        source_idat_payload.len(),
        "private JPEG Gain Map tmap",
    )?;
    let retained_idat = &source_idat_payload[..old_tmap_range.start];
    let tmap_payload = &source_idat_payload[old_tmap_range.clone()];
    let grid_payload = make_grid_payload(
        rows,
        columns,
        spec.gain_map_width,
        spec.gain_map_height,
    )?;
    let grid_idat_offset = u64::try_from(retained_idat.len())
        .map_err(|_| invalid("grid idat offset exceeds u64"))?;
    let tmap_idat_offset = grid_idat_offset
        .checked_add(u64::try_from(grid_payload.len()).map_err(|_| invalid("grid payload exceeds u64"))?)
        .ok_or_else(|| invalid("tmap idat offset overflows"))?;
    let mut idat_payload = retained_idat.to_vec();
    idat_payload.extend_from_slice(&grid_payload);
    idat_payload.extend_from_slice(tmap_payload);
    let idat_part = make_box(IDAT, &idat_payload)?;

    let mut output_refs = Vec::new();
    for reference in &refs.entries {
        if (reference.kind == DIMG && reference.from_item_id == gain_map_id)
            || (reference.kind == DIMG && reference.from_item_id == tmap_id)
            || reference.kind == AUXL
            || xmp_id.is_some_and(|id| reference.from_item_id == id)
        {
            continue;
        }
        let mapped_from = output_item_id(reference.from_item_id);
        let mut mapped_targets: Vec<u32> = reference
            .to_item_ids
            .iter()
            .copied()
            .map(output_item_id)
            .collect();
        if reference.kind == CDSC
            && item_by_id
                .get(&reference.from_item_id)
                .is_some_and(|item| item_is(item, EXIF))
            && reference.to_item_ids.contains(&primary_id)
            && !reference.to_item_ids.contains(&tmap_id)
        {
            mapped_targets.push(output_tmap_id);
        }
        output_refs.push(IrefEntry {
            kind: reference.kind,
            from_item_id: mapped_from,
            to_item_ids: mapped_targets,
        });
    }
    output_refs.push(IrefEntry {
        kind: DIMG,
        from_item_id: output_gain_map_id,
        to_item_ids: tile_ids.clone(),
    });
    output_refs.push(IrefEntry {
        kind: DIMG,
        from_item_id: output_tmap_id,
        to_item_ids: vec![primary_id, output_gain_map_id],
    });
    let maximum_ref_id = output_refs
        .iter()
        .flat_map(|reference| std::iter::once(reference.from_item_id).chain(reference.to_item_ids.iter().copied()))
        .max()
        .unwrap_or(0);
    let iref_version = if maximum_ref_id > u32::from(u16::MAX) {
        1
    } else {
        refs.version
    };
    let iref_part = make_iref_box(iref_version, &output_refs)?;

    let mut placeholder_locations = Vec::new();
    for entry in &iloc.entries {
        if appended_graph_ids.contains(&entry.item_id) && entry.item_id != tmap_id {
            continue;
        }
        let extents = if entry.item_id == tmap_id {
            vec![IlocExtent {
                index: None,
                offset: tmap_idat_offset,
                length: old_tmap_extent.length,
            }]
        } else {
            entry
                .extents
                .iter()
                .map(|extent| IlocExtent {
                    index: None,
                    offset: 0,
                    length: extent.length,
                })
                .collect()
        };
        placeholder_locations.push(IlocEntry {
            item_id: output_item_id(entry.item_id),
            construction_method: entry.construction_method,
            data_reference_index: entry.data_reference_index,
            base_offset: 0,
            extents,
        });
    }
    placeholder_locations.push(IlocEntry {
        item_id: output_gain_map_id,
        construction_method: 1,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: grid_idat_offset,
            length: u64::try_from(grid_payload.len()).map_err(|_| invalid("grid payload exceeds u64"))?,
        }],
    });
    for (tile_id, tile) in tile_ids.iter().copied().zip(spec.tiles.iter().copied()) {
        placeholder_locations.push(IlocEntry {
            item_id: tile_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: u64::try_from(tile.payload.len())
                    .map_err(|_| invalid("tile payload exceeds u64"))?,
            }],
        });
    }
    placeholder_locations.sort_by_key(|entry| entry.item_id);
    let placeholder_iloc = make_iloc_box(1, 4, 4, 0, 0, &placeholder_locations)?;
    let grpl_part = make_altr_group(
        output_tmap_id
            .checked_add(1)
            .ok_or_else(|| invalid("altr group ID overflows"))?,
        output_tmap_id,
        primary_id,
    )?;

    let mut meta_parts: Vec<Vec<u8>> = Vec::new();
    for part in &meta_children {
        let replacement = if part.kind == IINF {
            Some(iinf_part.clone())
        } else if part.kind == ILOC {
            Some(placeholder_iloc.clone())
        } else if part.kind == IPRP {
            Some(iprp_part.clone())
        } else if part.kind == IREF {
            Some(iref_part.clone())
        } else if part.kind == IDAT {
            Some(idat_part.clone())
        } else if part.kind == GRPL {
            Some(grpl_part.clone())
        } else {
            None
        };
        meta_parts.push(match replacement {
            Some(bytes) => bytes,
            None => raw_box(source, part, "meta child")?.to_vec(),
        });
    }

    let ftyp_part = build_ftyp(source, ftyp)?;
    let meta_full_header = source
        .get(meta.data_start..meta_children_start)
        .ok_or_else(|| invalid("meta full-box header is outside source"))?;
    let mut preliminary_meta_payload = meta_full_header.to_vec();
    for part in &meta_parts {
        preliminary_meta_payload.extend_from_slice(part);
    }
    let preliminary_meta = make_box(META, &preliminary_meta_payload)?;
    if meta.data_end > mdat.box_start {
        return Err(invalid("meta box overlaps or follows mdat"));
    }
    let between_meta_and_mdat = source
        .get(meta.data_end..mdat.box_start)
        .ok_or_else(|| invalid("bytes between meta and mdat are outside source"))?;
    let new_mdat_data_start = ftyp_part
        .len()
        .checked_add(preliminary_meta.len())
        .and_then(|value| value.checked_add(between_meta_and_mdat.len()))
        .and_then(|value| value.checked_add(8))
        .ok_or_else(|| invalid("output mdat offset overflows"))?;
    let file_delta = i128::try_from(new_mdat_data_start)
        .map_err(|_| invalid("output mdat offset exceeds i128"))?
        - i128::try_from(mdat.data_start).map_err(|_| invalid("source mdat offset exceeds i128"))?;

    let jpeg_offset_usize = usize::try_from(jpeg_offset)
        .map_err(|_| invalid("private JPEG Gain Map offset exceeds usize"))?;
    if jpeg_offset_usize < mdat.data_start || jpeg_offset_usize > mdat.data_end {
        return Err(invalid("private JPEG Gain Map offset is outside mdat"));
    }
    let source_mdat_payload = &source[mdat.data_start..jpeg_offset_usize];

    let mut final_locations = Vec::new();
    for entry in &iloc.entries {
        if appended_graph_ids.contains(&entry.item_id) && entry.item_id != tmap_id {
            continue;
        }
        let extents = if entry.item_id == tmap_id {
            vec![IlocExtent {
                index: None,
                offset: tmap_idat_offset,
                length: old_tmap_extent.length,
            }]
        } else {
            resolved_extents(entry)?
                .into_iter()
                .map(|(offset, length)| {
                    let offset = if entry.construction_method == 0 {
                        shift_offset(offset, file_delta)?
                    } else {
                        offset
                    };
                    Ok(IlocExtent {
                        index: None,
                        offset,
                        length,
                    })
                })
                .collect::<Result<Vec<_>>>()?
        };
        final_locations.push(IlocEntry {
            item_id: output_item_id(entry.item_id),
            construction_method: entry.construction_method,
            data_reference_index: entry.data_reference_index,
            base_offset: 0,
            extents,
        });
    }
    final_locations.push(IlocEntry {
        item_id: output_gain_map_id,
        construction_method: 1,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: grid_idat_offset,
            length: u64::try_from(grid_payload.len()).map_err(|_| invalid("grid payload exceeds u64"))?,
        }],
    });

    let mut appended_tile_bytes = 0usize;
    for (tile_id, tile) in tile_ids.iter().copied().zip(spec.tiles.iter().copied()) {
        let absolute = new_mdat_data_start
            .checked_add(source_mdat_payload.len())
            .and_then(|value| value.checked_add(appended_tile_bytes))
            .ok_or_else(|| invalid("tile mdat offset overflows"))?;
        final_locations.push(IlocEntry {
            item_id: tile_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: u64::try_from(absolute).map_err(|_| invalid("tile mdat offset exceeds u64"))?,
                length: u64::try_from(tile.payload.len()).map_err(|_| invalid("tile payload exceeds u64"))?,
            }],
        });
        appended_tile_bytes = appended_tile_bytes
            .checked_add(tile.payload.len())
            .ok_or_else(|| invalid("appended tile payload length overflows"))?;
    }
    final_locations.sort_by_key(|entry| entry.item_id);
    let final_iloc = make_iloc_box(1, 4, 4, 0, 0, &final_locations)?;

    let mut final_meta_payload = meta_full_header.to_vec();
    for part in &meta_parts {
        if part.len() >= 8 && &part[4..8] == b"iloc" {
            final_meta_payload.extend_from_slice(&final_iloc);
        } else {
            final_meta_payload.extend_from_slice(part);
        }
    }
    let final_meta = make_box(META, &final_meta_payload)?;

    let mut final_mdat_payload = source_mdat_payload.to_vec();
    for tile in spec.tiles {
        final_mdat_payload.extend_from_slice(tile.payload);
    }
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;

    let mut output = Vec::new();
    output.extend_from_slice(&ftyp_part);
    output.extend_from_slice(&final_meta);
    output.extend_from_slice(between_meta_and_mdat);
    output.extend_from_slice(&final_mdat);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_hvcc(channel_count: u8) -> Vec<u8> {
        let mut hvcc = vec![0u8; 19];
        hvcc[0] = 1;
        hvcc[1] = 4;
        hvcc[16] = if channel_count == 1 { 0 } else { 3 };
        hvcc
    }

    #[test]
    fn rejects_bad_tile_geometry_before_parsing_source() {
        let tile = GainMapTile {
            payload: &[1],
            width: 3,
            height: 4,
        };
        let hvcc = valid_hvcc(3);
        let spec = DirectHevcGainMap {
            gain_map_width: 4,
            gain_map_height: 4,
            tile_width: 4,
            tile_height: 4,
            tiles: &[tile],
            hvcc: &hvcc,
            channel_count: 3,
        };
        assert!(replace_private_jpeg_gain_map_with_hevc_tiles(&[], &spec).is_err());
    }

    #[test]
    fn rejects_codec_channel_mismatch_before_parsing_source() {
        let tile = GainMapTile {
            payload: &[1],
            width: 4,
            height: 4,
        };
        let hvcc = valid_hvcc(1);
        let spec = DirectHevcGainMap {
            gain_map_width: 4,
            gain_map_height: 4,
            tile_width: 4,
            tile_height: 4,
            tiles: &[tile],
            hvcc: &hvcc,
            channel_count: 3,
        };
        assert!(replace_private_jpeg_gain_map_with_hevc_tiles(&[], &spec).is_err());
    }

    #[test]
    fn grid_payload_uses_large_dimension_form_only_when_required() {
        assert_eq!(
            make_grid_payload(2, 3, 640, 480).unwrap(),
            vec![0, 0, 1, 2, 0x02, 0x80, 0x01, 0xe0]
        );
        assert_eq!(
            make_grid_payload(1, 1, 65_536, 70_000).unwrap(),
            vec![0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0x11, 0x70]
        );
    }
}

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use xdremux_format::isobmff::{
    parse_boxes, parse_ispe_dimensions, parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry,
    IpmaEntry, ParsedMeta, PropertyInfo, FTYP, MDAT, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const GRID: FourCC = FourCC::new(*b"grid");
const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const TMAP: FourCC = FourCC::new(*b"tmap");
const ISPE: FourCC = FourCC::new(*b"ispe");
const PIXI: FourCC = FourCC::new(*b"pixi");
const DIMG: FourCC = FourCC::new(*b"dimg");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainMapStructure {
    pub primary_item_id: u32,
    pub tmap_item_id: u32,
    pub gain_map_item_id: u32,
    pub tile_item_ids: Vec<u32>,
    pub width: u32,
    pub height: u32,
    pub rows: u32,
    pub columns: u32,
    pub channel_count: u8,
    pub chroma_format_idc: u8,
    pub bit_depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CodecLayout {
    chroma_format_idc: u8,
    bit_depth_luma: u8,
    bit_depth_chroma: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GridGeometry {
    rows: u32,
    columns: u32,
    width: u32,
    height: u32,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn one_top_level<'a>(
    boxes: &'a [BoxHeader],
    kind: FourCC,
    context: &str,
) -> Result<&'a BoxHeader> {
    let mut matches = boxes.iter().filter(|header| header.kind == kind);
    let Some(first) = matches.next() else {
        return Err(invalid(format!("{context} is missing")));
    };
    if matches.next().is_some() {
        return Err(invalid(format!("{context} appears more than once")));
    }
    Ok(first)
}

fn property_box<'a>(source: &'a [u8], property: &PropertyInfo) -> Result<(&'a [u8], BoxHeader)> {
    let raw = source
        .get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside the HEIF", property.index)))?;
    let headers = parse_boxes(raw, 0..raw.len())?;
    if headers.len() != 1 {
        return Err(invalid(format!(
            "property {} does not contain exactly one box",
            property.index
        )));
    }
    let header = headers[0].clone();
    if header.kind != property.kind {
        return Err(invalid(format!(
            "property {} parsed as {}, expected {}",
            property.index, header.kind, property.kind
        )));
    }
    Ok((raw, header))
}

fn parse_pixi(source: &[u8], property: &PropertyInfo) -> Result<(u8, Vec<u8>)> {
    let (raw, header) = property_box(source, property)?;
    if header.kind != PIXI {
        return Err(invalid(format!(
            "property {} is {}, expected pixi",
            property.index, header.kind
        )));
    }
    let payload = raw
        .get(header.payload_range())
        .ok_or_else(|| invalid("pixi payload is outside its property box"))?;
    if payload.len() < 5 || payload[..4] != [0, 0, 0, 0] {
        return Err(invalid("pixi must use version 0 with zero flags"));
    }
    let channel_count = payload[4];
    if channel_count == 0 {
        return Err(invalid("pixi declares zero channels"));
    }
    let expected = usize::from(channel_count)
        .checked_add(5)
        .ok_or_else(|| invalid("pixi channel count overflows"))?;
    if payload.len() != expected {
        return Err(invalid(format!(
            "pixi declares {channel_count} channels but has {} payload bytes",
            payload.len()
        )));
    }
    Ok((channel_count, payload[5..].to_vec()))
}

fn parse_ispe(source: &[u8], property: &PropertyInfo) -> Result<(u32, u32)> {
    let (raw, header) = property_box(source, property)?;
    if header.kind != ISPE {
        return Err(invalid(format!(
            "property {} is {}, expected ispe",
            property.index, header.kind
        )));
    }
    let (width, height) = parse_ispe_dimensions(raw, &header)?;
    if width == 0 || height == 0 {
        return Err(invalid("ispe dimensions must be non-zero"));
    }
    Ok((width, height))
}

fn parse_hvcc(source: &[u8], property: &PropertyInfo) -> Result<CodecLayout> {
    let (raw, header) = property_box(source, property)?;
    if header.kind != HVCC {
        return Err(invalid(format!(
            "property {} is {}, expected hvcC",
            property.index, header.kind
        )));
    }
    let payload = raw
        .get(header.payload_range())
        .ok_or_else(|| invalid("hvcC payload is outside its property box"))?;
    if payload.len() <= 18 {
        return Err(invalid("hvcC is shorter than the channel-layout fields"));
    }
    if payload[0] != 1 {
        return Err(invalid(format!(
            "hvcC configurationVersion {} is unsupported",
            payload[0]
        )));
    }
    if payload[1] & 0x1f != 4 {
        return Err(invalid(format!(
            "hvcC general_profile_idc {} does not match the direct Gain Map contract",
            payload[1] & 0x1f
        )));
    }
    let chroma_format_idc = payload[16] & 0x03;
    if !matches!(chroma_format_idc, 0 | 3) {
        return Err(invalid(format!(
            "hvcC chroma_format_idc {chroma_format_idc} is unsupported for a Gain Map"
        )));
    }
    let bit_depth_luma = (payload[17] & 0x07) + 8;
    let bit_depth_chroma = (payload[18] & 0x07) + 8;
    if bit_depth_luma != 8 || bit_depth_chroma != 8 {
        return Err(invalid(format!(
            "hvcC Gain Map bit depth must be 8/8, got {bit_depth_luma}/{bit_depth_chroma}"
        )));
    }
    Ok(CodecLayout {
        chroma_format_idc,
        bit_depth_luma,
        bit_depth_chroma,
    })
}

fn item_extent_range(
    source_len: usize,
    idat: Option<&BoxHeader>,
    entry: &IlocEntry,
    extent_offset: u64,
    extent_length: u64,
) -> Result<Range<usize>> {
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "item {} uses unsupported data_reference_index {}",
            entry.item_id, entry.data_reference_index
        )));
    }
    let relative = entry
        .base_offset
        .checked_add(extent_offset)
        .ok_or_else(|| invalid(format!("item {} extent offset overflows", entry.item_id)))?;
    let length = usize::try_from(extent_length)
        .map_err(|_| invalid(format!("item {} extent length exceeds usize", entry.item_id)))?;
    let start = match entry.construction_method {
        0 => usize::try_from(relative)
            .map_err(|_| invalid(format!("item {} file offset exceeds usize", entry.item_id)))?,
        1 => {
            let idat = idat.ok_or_else(|| {
                invalid(format!(
                    "item {} uses idat construction without an idat box",
                    entry.item_id
                ))
            })?;
            let relative = usize::try_from(relative).map_err(|_| {
                invalid(format!("item {} idat offset exceeds usize", entry.item_id))
            })?;
            if relative > idat.payload_range().len() {
                return Err(invalid(format!(
                    "item {} idat extent starts outside idat",
                    entry.item_id
                )));
            }
            idat.data_start
                .checked_add(relative)
                .ok_or_else(|| invalid(format!("item {} idat offset overflows", entry.item_id)))?
        }
        method => {
            return Err(invalid(format!(
                "item {} uses unsupported construction_method {method}",
                entry.item_id
            )));
        }
    };
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid(format!("item {} extent end overflows", entry.item_id)))?;
    let limit = if entry.construction_method == 1 {
        idat.expect("construction method checked").data_end
    } else {
        source_len
    };
    if end > limit {
        return Err(invalid(format!(
            "item {} extent {}..{} exceeds its backing storage ending at {}",
            entry.item_id, start, end, limit
        )));
    }
    Ok(start..end)
}

fn item_payload(source: &[u8], idat: Option<&BoxHeader>, entry: &IlocEntry) -> Result<Vec<u8>> {
    let total = entry.extents.iter().try_fold(0usize, |total, extent| {
        let length = usize::try_from(extent.length)
            .map_err(|_| invalid(format!("item {} extent length exceeds usize", entry.item_id)))?;
        total
            .checked_add(length)
            .ok_or_else(|| invalid(format!("item {} payload length overflows", entry.item_id)))
    })?;
    let mut payload = Vec::with_capacity(total);
    for extent in &entry.extents {
        let range = item_extent_range(
            source.len(),
            idat,
            entry,
            extent.offset,
            extent.length,
        )?;
        payload.extend_from_slice(
            source
                .get(range)
                .ok_or_else(|| invalid(format!("item {} extent is outside the HEIF", entry.item_id)))?,
        );
    }
    Ok(payload)
}

fn parse_grid(payload: &[u8]) -> Result<GridGeometry> {
    if payload.len() < 4 {
        return Err(invalid("grid payload is shorter than its header"));
    }
    if payload[0] != 0 {
        return Err(invalid(format!(
            "grid version {} is unsupported",
            payload[0]
        )));
    }
    if payload[1] & !1 != 0 {
        return Err(invalid(format!(
            "grid flags 0x{:02x} contain unsupported bits",
            payload[1]
        )));
    }
    let rows = u32::from(payload[2]) + 1;
    let columns = u32::from(payload[3]) + 1;
    let large = payload[1] & 1 != 0;
    let (width, height) = if large {
        if payload.len() != 12 {
            return Err(invalid("large grid payload must be exactly 12 bytes"));
        }
        (
            u32::from_be_bytes(payload[4..8].try_into().expect("checked length")),
            u32::from_be_bytes(payload[8..12].try_into().expect("checked length")),
        )
    } else {
        if payload.len() != 8 {
            return Err(invalid("small grid payload must be exactly 8 bytes"));
        }
        (
            u32::from(u16::from_be_bytes(
                payload[4..6].try_into().expect("checked length"),
            )),
            u32::from(u16::from_be_bytes(
                payload[6..8].try_into().expect("checked length"),
            )),
        )
    };
    if width == 0 || height == 0 {
        return Err(invalid("grid dimensions must be non-zero"));
    }
    Ok(GridGeometry {
        rows,
        columns,
        width,
        height,
    })
}

fn associated_property<'a>(
    item_id: u32,
    kind: FourCC,
    ipma_by_item: &HashMap<u32, &'a IpmaEntry>,
    properties: &HashMap<u32, &'a PropertyInfo>,
) -> Result<&'a PropertyInfo> {
    let entry = ipma_by_item
        .get(&item_id)
        .copied()
        .ok_or_else(|| invalid(format!("item {item_id} has no ipma entry")))?;
    let mut matches = entry.associations.iter().filter_map(|association| {
        properties
            .get(&u32::from(association.property_index))
            .copied()
            .filter(|property| property.kind == kind)
    });
    let Some(property) = matches.next() else {
        return Err(invalid(format!("item {item_id} is missing {kind}")));
    };
    if matches.next().is_some() {
        return Err(invalid(format!(
            "item {item_id} has more than one {kind} property"
        )));
    }
    Ok(property)
}

fn validate_meta_integrity<'a>(
    source: &[u8],
    meta: &'a ParsedMeta,
) -> Result<(
    HashMap<u32, &'a xdremux_format::isobmff::ItemInfo>,
    HashMap<u32, &'a IlocEntry>,
    HashMap<u32, &'a IpmaEntry>,
    HashMap<u32, &'a PropertyInfo>,
)> {
    let mut items = HashMap::new();
    for item in &meta.iinf.entries {
        if items.insert(item.item_id, item).is_some() {
            return Err(invalid(format!("duplicate iinf item ID {}", item.item_id)));
        }
    }
    if !items.contains_key(&meta.primary_item_id) {
        return Err(invalid(format!(
            "pitm references unknown item {}",
            meta.primary_item_id
        )));
    }

    let mut locations = HashMap::new();
    for entry in &meta.iloc.entries {
        if !items.contains_key(&entry.item_id) {
            return Err(invalid(format!(
                "iloc references unknown item {}",
                entry.item_id
            )));
        }
        if locations.insert(entry.item_id, entry).is_some() {
            return Err(invalid(format!("duplicate iloc item ID {}", entry.item_id)));
        }
        for extent in &entry.extents {
            let _ = item_extent_range(
                source.len(),
                meta.idat.as_ref(),
                entry,
                extent.offset,
                extent.length,
            )?;
        }
    }

    let properties: HashMap<u32, &PropertyInfo> =
        meta.properties.iter().map(|property| (property.index, property)).collect();
    if properties.len() != meta.properties.len() {
        return Err(invalid("duplicate ipco property index"));
    }

    let mut ipma_by_item = HashMap::new();
    for entry in &meta.ipma.entries {
        if !items.contains_key(&entry.item_id) {
            return Err(invalid(format!(
                "ipma references unknown item {}",
                entry.item_id
            )));
        }
        if ipma_by_item.insert(entry.item_id, entry).is_some() {
            return Err(invalid(format!("duplicate ipma item ID {}", entry.item_id)));
        }
        let mut seen = HashSet::new();
        for association in &entry.associations {
            if association.property_index == 0 {
                return Err(invalid(format!(
                    "item {} has an ipma association to property index 0",
                    entry.item_id
                )));
            }
            let index = u32::from(association.property_index);
            if !seen.insert(index) {
                return Err(invalid(format!(
                    "item {} repeats ipma property index {index}",
                    entry.item_id
                )));
            }
            if !properties.contains_key(&index) {
                return Err(invalid(format!(
                    "item {} references missing ipco property {index}",
                    entry.item_id
                )));
            }
        }
    }

    if let Some(iref) = &meta.iref {
        for reference in &iref.entries {
            if !items.contains_key(&reference.from_item_id) {
                return Err(invalid(format!(
                    "{} reference originates from unknown item {}",
                    reference.kind, reference.from_item_id
                )));
            }
            for target in &reference.to_item_ids {
                if !items.contains_key(target) {
                    return Err(invalid(format!(
                        "{} reference from item {} targets unknown item {target}",
                        reference.kind, reference.from_item_id
                    )));
                }
            }
        }
    }

    Ok((items, locations, ipma_by_item, properties))
}

/// Validates the portable ISO-BMFF/HEVC structure emitted by XDRemux's direct
/// tiled Gain Map writer. This intentionally does not claim Apple consumer
/// recognition; ImageIO/Photos validation remains an Apple-platform gate.
pub fn validate_gain_map_structure(source: &[u8]) -> Result<GainMapStructure> {
    let top = scan_top_level_boxes(source)?;
    let _ftyp = one_top_level(&top.boxes, FTYP, "ftyp")?;
    let meta_header = one_top_level(&top.boxes, META, "meta")?;
    let _mdat = one_top_level(&top.boxes, MDAT, "mdat")?;
    let meta = parse_meta_box(source, meta_header)?;
    let (items, locations, ipma_by_item, properties) = validate_meta_integrity(source, &meta)?;

    let tmap_items: Vec<_> = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(TMAP))
        .collect();
    if tmap_items.len() != 1 {
        return Err(invalid(format!(
            "expected exactly one tmap item, found {}",
            tmap_items.len()
        )));
    }
    let tmap_item_id = tmap_items[0].item_id;
    let iref = meta
        .iref
        .as_ref()
        .ok_or_else(|| invalid("tmap graph requires iref"))?;
    let tmap_refs: Vec<_> = iref
        .entries
        .iter()
        .filter(|reference| reference.kind == DIMG && reference.from_item_id == tmap_item_id)
        .collect();
    if tmap_refs.len() != 1 {
        return Err(invalid(format!(
            "tmap item {tmap_item_id} must have exactly one dimg reference"
        )));
    }
    let tmap_targets = &tmap_refs[0].to_item_ids;
    if tmap_targets.len() != 2 || tmap_targets[0] != meta.primary_item_id {
        return Err(invalid(format!(
            "tmap item {tmap_item_id} must dimg-reference [primary, gain-map] in that order"
        )));
    }
    let gain_map_item_id = tmap_targets[1];
    let gain_map_item = items
        .get(&gain_map_item_id)
        .copied()
        .ok_or_else(|| invalid(format!("gain-map item {gain_map_item_id} is missing")))?;
    if gain_map_item.item_type != Some(GRID) {
        return Err(invalid(format!(
            "gain-map item {gain_map_item_id} is {:?}, expected grid",
            gain_map_item.item_type
        )));
    }

    let gain_refs: Vec<_> = iref
        .entries
        .iter()
        .filter(|reference| reference.kind == DIMG && reference.from_item_id == gain_map_item_id)
        .collect();
    if gain_refs.len() != 1 || gain_refs[0].to_item_ids.is_empty() {
        return Err(invalid(format!(
            "grid gain-map item {gain_map_item_id} must have exactly one non-empty dimg reference"
        )));
    }
    let tile_item_ids = gain_refs[0].to_item_ids.clone();
    let unique_tiles: HashSet<_> = tile_item_ids.iter().copied().collect();
    if unique_tiles.len() != tile_item_ids.len() {
        return Err(invalid("gain-map dimg reference repeats a tile item ID"));
    }

    let gain_location = locations
        .get(&gain_map_item_id)
        .copied()
        .ok_or_else(|| invalid(format!("gain-map item {gain_map_item_id} has no iloc entry")))?;
    let grid = parse_grid(&item_payload(source, meta.idat.as_ref(), gain_location)?)?;
    let expected_tiles = grid
        .rows
        .checked_mul(grid.columns)
        .ok_or_else(|| invalid("grid tile count overflows"))?;
    if usize::try_from(expected_tiles).ok() != Some(tile_item_ids.len()) {
        return Err(invalid(format!(
            "grid declares {}x{} tiles but dimg contains {} items",
            grid.rows,
            grid.columns,
            tile_item_ids.len()
        )));
    }

    let gain_ispe = associated_property(
        gain_map_item_id,
        ISPE,
        &ipma_by_item,
        &properties,
    )?;
    let (ispe_width, ispe_height) = parse_ispe(source, gain_ispe)?;
    if (ispe_width, ispe_height) != (grid.width, grid.height) {
        return Err(invalid(format!(
            "grid payload dimensions {}x{} disagree with ispe {}x{}",
            grid.width, grid.height, ispe_width, ispe_height
        )));
    }
    let gain_pixi = associated_property(
        gain_map_item_id,
        PIXI,
        &ipma_by_item,
        &properties,
    )?;
    let (channel_count, channel_bits) = parse_pixi(source, gain_pixi)?;
    if !matches!(channel_count, 1 | 3) || channel_bits.iter().any(|bits| *bits != 8) {
        return Err(invalid(format!(
            "gain-map pixi must declare one or three 8-bit channels, got {channel_count} {:?}",
            channel_bits
        )));
    }

    let _ = parse_ispe(
        source,
        associated_property(tmap_item_id, ISPE, &ipma_by_item, &properties)?,
    )?;
    let _ = parse_pixi(
        source,
        associated_property(tmap_item_id, PIXI, &ipma_by_item, &properties)?,
    )?;

    let mut expected_codec: Option<CodecLayout> = None;
    for tile_item_id in &tile_item_ids {
        let tile_item = items
            .get(tile_item_id)
            .copied()
            .ok_or_else(|| invalid(format!("tile item {tile_item_id} is missing")))?;
        if tile_item.item_type != Some(HVC1) {
            return Err(invalid(format!(
                "gain-map tile {tile_item_id} is {:?}, expected hvc1",
                tile_item.item_type
            )));
        }
        let _ = locations
            .get(tile_item_id)
            .copied()
            .ok_or_else(|| invalid(format!("tile item {tile_item_id} has no iloc entry")))?;
        let _ = parse_ispe(
            source,
            associated_property(*tile_item_id, ISPE, &ipma_by_item, &properties)?,
        )?;
        let codec = parse_hvcc(
            source,
            associated_property(*tile_item_id, HVCC, &ipma_by_item, &properties)?,
        )?;
        if let Some(expected) = expected_codec {
            if codec != expected {
                return Err(invalid(format!(
                    "tile item {tile_item_id} uses a different hvcC channel layout"
                )));
            }
        } else {
            expected_codec = Some(codec);
        }
    }
    let codec = expected_codec.ok_or_else(|| invalid("gain-map contains no HEVC tiles"))?;
    let codec_channels = if codec.chroma_format_idc == 0 { 1 } else { 3 };
    if channel_count != codec_channels {
        return Err(invalid(format!(
            "gain-map pixi declares {channel_count} channels but hvcC chroma_format_idc {} implies {codec_channels}",
            codec.chroma_format_idc
        )));
    }

    Ok(GainMapStructure {
        primary_item_id: meta.primary_item_id,
        tmap_item_id,
        gain_map_item_id,
        tile_item_ids,
        width: grid.width,
        height: grid.height,
        rows: grid.rows,
        columns: grid.columns,
        channel_count,
        chroma_format_idc: codec.chroma_format_idc,
        bit_depth: codec.bit_depth_luma,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{
        make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
        make_iref_box, make_ispe_box, make_pitm_box, IlocExtent, IpmaAssociation, IrefEntry,
        IPCO, IDAT, IPRP,
    };

    const PRIMARY_ID: u32 = 1;
    const GAIN_ID: u32 = 2;
    const TMAP_ID: u32 = 3;
    const TILE_ID: u32 = 4;

    struct FixtureOptions {
        tmap_targets: Vec<u32>,
        gain_targets: Vec<u32>,
        gain_pixi_channels: u8,
        hvcc_chroma: u8,
        tile_extent_length: u64,
        grid_rows: u8,
        grid_columns: u8,
    }

    impl Default for FixtureOptions {
        fn default() -> Self {
            Self {
                tmap_targets: vec![PRIMARY_ID, GAIN_ID],
                gain_targets: vec![TILE_ID],
                gain_pixi_channels: 3,
                hvcc_chroma: 3,
                tile_extent_length: 1,
                grid_rows: 1,
                grid_columns: 1,
            }
        }
    }

    fn pixi_box(channels: u8) -> Vec<u8> {
        let mut payload = vec![channels];
        payload.extend(std::iter::repeat_n(8, usize::from(channels)));
        make_full_box(PIXI, 0, 0, &payload).unwrap()
    }

    fn hvcc_box(chroma: u8) -> Vec<u8> {
        let mut payload = vec![0u8; 19];
        payload[0] = 1;
        payload[1] = 4;
        payload[16] = chroma;
        make_box(HVCC, &payload).unwrap()
    }

    fn fixture(options: FixtureOptions) -> Vec<u8> {
        let primary_payload = [0xa1];
        let mut grid_payload = vec![
            0,
            0,
            options.grid_rows - 1,
            options.grid_columns - 1,
        ];
        grid_payload.extend_from_slice(&4u16.to_be_bytes());
        grid_payload.extend_from_slice(&4u16.to_be_bytes());
        let tmap_payload = [0xb2];
        let tile_payload = [0xc3];
        let mut idat_payload = Vec::new();
        let primary_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&primary_payload);
        let gain_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&grid_payload);
        let tmap_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&tmap_payload);
        let tile_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&tile_payload);

        let gain_ispe = make_ispe_box(4, 4).unwrap();
        let gain_pixi = pixi_box(options.gain_pixi_channels);
        let tmap_ispe = make_ispe_box(4, 4).unwrap();
        let tmap_pixi = pixi_box(3);
        let tile_ispe = make_ispe_box(4, 4).unwrap();
        let tile_hvcc = hvcc_box(options.hvcc_chroma);
        let mut ipco_payload = Vec::new();
        for property in [
            &gain_ispe,
            &gain_pixi,
            &tmap_ispe,
            &tmap_pixi,
            &tile_ispe,
            &tile_hvcc,
        ] {
            ipco_payload.extend_from_slice(property);
        }
        let ipco = make_box(IPCO, &ipco_payload).unwrap();
        let ipma = make_ipma_box(
            0,
            0,
            &[
                IpmaEntry {
                    item_id: GAIN_ID,
                    associations: vec![
                        IpmaAssociation {
                            property_index: 1,
                            essential: false,
                        },
                        IpmaAssociation {
                            property_index: 2,
                            essential: false,
                        },
                    ],
                },
                IpmaEntry {
                    item_id: TMAP_ID,
                    associations: vec![
                        IpmaAssociation {
                            property_index: 3,
                            essential: false,
                        },
                        IpmaAssociation {
                            property_index: 4,
                            essential: false,
                        },
                    ],
                },
                IpmaEntry {
                    item_id: TILE_ID,
                    associations: vec![
                        IpmaAssociation {
                            property_index: 5,
                            essential: true,
                        },
                        IpmaAssociation {
                            property_index: 6,
                            essential: true,
                        },
                    ],
                },
            ],
        )
        .unwrap();
        let mut iprp_payload = ipco;
        iprp_payload.extend_from_slice(&ipma);
        let iprp = make_box(IPRP, &iprp_payload).unwrap();

        let infes = [
            make_infe_box(PRIMARY_ID, HVC1, 0).unwrap(),
            make_infe_box(GAIN_ID, GRID, 1).unwrap(),
            make_infe_box(TMAP_ID, TMAP, 0).unwrap(),
            make_infe_box(TILE_ID, HVC1, 1).unwrap(),
        ];
        let iinf = make_iinf_box(0, &infes).unwrap();
        let pitm = make_pitm_box(0, PRIMARY_ID).unwrap();
        let iloc = make_iloc_box(
            1,
            4,
            4,
            0,
            0,
            &[
                IlocEntry {
                    item_id: PRIMARY_ID,
                    construction_method: 1,
                    data_reference_index: 0,
                    base_offset: 0,
                    extents: vec![IlocExtent {
                        index: None,
                        offset: primary_offset,
                        length: 1,
                    }],
                },
                IlocEntry {
                    item_id: GAIN_ID,
                    construction_method: 1,
                    data_reference_index: 0,
                    base_offset: 0,
                    extents: vec![IlocExtent {
                        index: None,
                        offset: gain_offset,
                        length: grid_payload.len() as u64,
                    }],
                },
                IlocEntry {
                    item_id: TMAP_ID,
                    construction_method: 1,
                    data_reference_index: 0,
                    base_offset: 0,
                    extents: vec![IlocExtent {
                        index: None,
                        offset: tmap_offset,
                        length: 1,
                    }],
                },
                IlocEntry {
                    item_id: TILE_ID,
                    construction_method: 1,
                    data_reference_index: 0,
                    base_offset: 0,
                    extents: vec![IlocExtent {
                        index: None,
                        offset: tile_offset,
                        length: options.tile_extent_length,
                    }],
                },
            ],
        )
        .unwrap();
        let iref = make_iref_box(
            0,
            &[
                IrefEntry {
                    kind: DIMG,
                    from_item_id: TMAP_ID,
                    to_item_ids: options.tmap_targets,
                },
                IrefEntry {
                    kind: DIMG,
                    from_item_id: GAIN_ID,
                    to_item_ids: options.gain_targets,
                },
            ],
        )
        .unwrap();
        let idat = make_box(IDAT, &idat_payload).unwrap();

        let mut meta_payload = vec![0, 0, 0, 0];
        for child in [pitm, iinf, iloc, iprp, iref, idat] {
            meta_payload.extend_from_slice(&child);
        }
        let meta = make_box(META, &meta_payload).unwrap();
        let ftyp = make_box(FTYP, b"mif1\0\0\0\0").unwrap();
        let mdat = make_box(MDAT, &[]).unwrap();
        [ftyp, meta, mdat].concat()
    }

    #[test]
    fn accepts_valid_tiled_gain_map_graph() {
        let result = validate_gain_map_structure(&fixture(FixtureOptions::default())).unwrap();
        assert_eq!(result.primary_item_id, PRIMARY_ID);
        assert_eq!(result.tmap_item_id, TMAP_ID);
        assert_eq!(result.gain_map_item_id, GAIN_ID);
        assert_eq!(result.tile_item_ids, vec![TILE_ID]);
        assert_eq!((result.width, result.height), (4, 4));
        assert_eq!((result.rows, result.columns), (1, 1));
        assert_eq!(result.channel_count, 3);
        assert_eq!(result.chroma_format_idc, 3);
        assert_eq!(result.bit_depth, 8);
    }

    #[test]
    fn rejects_tmap_reference_to_unknown_gain_map() {
        let options = FixtureOptions {
            tmap_targets: vec![PRIMARY_ID, 99],
            ..FixtureOptions::default()
        };
        assert!(validate_gain_map_structure(&fixture(options)).is_err());
    }

    #[test]
    fn rejects_idat_extent_past_storage() {
        let options = FixtureOptions {
            tile_extent_length: 2,
            ..FixtureOptions::default()
        };
        assert!(validate_gain_map_structure(&fixture(options)).is_err());
    }

    #[test]
    fn rejects_pixi_hvcc_channel_mismatch() {
        let options = FixtureOptions {
            gain_pixi_channels: 3,
            hvcc_chroma: 0,
            ..FixtureOptions::default()
        };
        assert!(validate_gain_map_structure(&fixture(options)).is_err());
    }

    #[test]
    fn rejects_grid_tile_count_mismatch() {
        let options = FixtureOptions {
            grid_rows: 1,
            grid_columns: 2,
            ..FixtureOptions::default()
        };
        assert!(validate_gain_map_structure(&fixture(options)).is_err());
    }
}

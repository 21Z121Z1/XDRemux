use std::collections::{HashMap, HashSet};
use std::ops::Range;

use xdremux_format::isobmff::{
    parse_boxes, parse_ispe_dimensions, parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry,
    IpmaEntry, ParsedMeta, PropertyInfo, FTYP, MDAT, META,
};
use xdremux_format::{parse_hvcc_profile, ChromaSampling, FourCC, HevcDecoderConfigurationProfile};

use crate::error::{HeifError, Result};

const GRID: FourCC = FourCC::new(*b"grid");
const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const TMAP: FourCC = FourCC::new(*b"tmap");
const ISPE: FourCC = FourCC::new(*b"ispe");
const PIXI: FourCC = FourCC::new(*b"pixi");
const DIMG: FourCC = FourCC::new(*b"dimg");
const COLR: FourCC = FourCC::new(*b"colr");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainMapStructure {
    pub primary_item_id: u32,
    pub tmap_item_id: u32,
    pub gain_map_item_id: u32,
    /// Actual coded HEVC resource IDs in image order. A direct hvc1 Gain Map
    /// contains its own item ID; a grid contains its dimg tile IDs.
    pub tile_item_ids: Vec<u32>,
    pub width: u32,
    pub height: u32,
    /// Coded resource layout; direct hvc1 reports 1x1 without authoring a grid.
    pub rows: u32,
    pub columns: u32,
    pub channel_count: u8,
    pub chroma_format_idc: u8,
    pub bit_depth: u8,
    pub chroma_sampling: ChromaSampling,
    pub luma_bit_depth: u8,
    pub chroma_bit_depth: u8,
    pub general_profile_idc: u8,
}

/// Observed metadata and output colorimetry from a complete HEIF gain-map graph.
/// Pixel decoding and platform recognition are separate verification gates.
#[derive(Debug, Clone, PartialEq)]
pub struct GainMapSemantics {
    pub structure: GainMapStructure,
    pub metadata: xdremux_metadata::UltraHdrGainMapMetadata,
    pub output_color: HdrOutputColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HdrOutputColor {
    Nclx {
        primaries: u16,
        transfer: u16,
        matrix: u16,
        full_range: bool,
    },
    Icc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HdrProfileSemantics {
    pub semantics: GainMapSemantics,
    pub base_color_space: xdremux_metadata::SdrColorSpace,
    pub static_metadata: xdremux_metadata::HdrStaticMetadata,
    pub registered_metadata: Option<xdremux_metadata::RegisteredHdrMetadata>,
}

fn item_properties(meta: &ParsedMeta, item_id: u32) -> Result<Vec<(&PropertyInfo, bool)>> {
    let mut entries = meta.ipma.entries.iter().filter(|e| e.item_id == item_id);
    let entry = entries
        .next()
        .ok_or_else(|| invalid(format!("item {item_id} has no property associations")))?;
    if entries.next().is_some() {
        return Err(invalid("duplicate item property associations"));
    }
    entry
        .associations
        .iter()
        .map(|a| {
            meta.properties
                .iter()
                .find(|p| p.index == u32::from(a.property_index))
                .map(|p| (p, a.essential))
                .ok_or_else(|| invalid("association references missing property"))
        })
        .collect()
}

pub(crate) fn primary_image_items(meta: &ParsedMeta) -> Result<Vec<u32>> {
    let mut result = vec![meta.primary_item_id];
    let mut seen = HashSet::from([meta.primary_item_id]);
    let mut cursor = 0;
    while cursor < result.len() {
        let id = result[cursor];
        cursor += 1;
        if let Some(iref) = &meta.iref {
            for reference in iref
                .entries
                .iter()
                .filter(|r| r.kind == DIMG && r.from_item_id == id)
            {
                for target in &reference.to_item_ids {
                    if !meta.iinf.entries.iter().any(|i| i.item_id == *target) {
                        return Err(invalid("primary graph references a missing image"));
                    }
                    if seen.insert(*target) {
                        result.push(*target);
                    }
                }
            }
        }
    }
    Ok(result)
}

pub(crate) fn primary_color_property_indices(meta: &ParsedMeta) -> Result<Vec<u16>> {
    for id in primary_image_items(meta)? {
        let properties = item_properties(meta, id)?
            .into_iter()
            .filter(|(p, _)| p.kind == COLR)
            .map(|(p, _)| {
                u16::try_from(p.index).map_err(|_| invalid("color property index exceeds u16"))
            })
            .collect::<Result<Vec<_>>>()?;
        if !properties.is_empty() {
            return Ok(properties);
        }
    }
    Err(invalid("primary image tree has no color property"))
}

fn item_color_space(
    source: &[u8],
    meta: &ParsedMeta,
    item_id: u32,
) -> Result<Option<xdremux_metadata::SdrColorSpace>> {
    let mut icc = None;
    let mut nclx = None;
    for (p, _) in item_properties(meta, item_id)? {
        if p.kind != COLR {
            continue;
        }
        let (raw, header) = property_box(source, p)?;
        let payload = &raw[header.payload_range()];
        match payload.get(..4) {
            Some(b"prof" | b"rICC") => {
                if icc.replace(&payload[4..]).is_some() {
                    return Err(invalid("duplicate primary ICC profile"));
                }
            }
            Some(b"nclx") => {
                if payload.len() != 11 || payload[10] & 0x7f != 0 {
                    return Err(invalid("malformed primary nclx"));
                }
                let c = std::array::from_fn(|i| {
                    u16::from_be_bytes([payload[4 + 2 * i], payload[5 + 2 * i]])
                });
                if nclx.replace(c).is_some() {
                    return Err(invalid("duplicate primary nclx"));
                }
            }
            _ => return Err(invalid("unsupported primary color property")),
        }
    }
    if icc.is_none() && nclx.is_none_or(|c| c[0] == 2 && c[1] == 2) {
        return Ok(None);
    }
    xdremux_metadata::identify_sdr_color_space(icc, nclx)
        .map(Some)
        .map_err(|e| invalid(e.to_string()))
}

fn primary_color_space(
    source: &[u8],
    meta: &ParsedMeta,
) -> Result<xdremux_metadata::SdrColorSpace> {
    let mut color = None;
    let mut checked = HashSet::new();
    for id in primary_image_items(meta)? {
        let properties = item_properties(meta, id)?;
        if properties
            .iter()
            .any(|(p, _)| matches!(p.kind.as_bytes(), b"clap" | b"pasp"))
        {
            return Err(invalid(
                "BT.2020 HDR profiles do not support cropped or non-square source pixel geometry",
            ));
        }
        // Shared property indices refer to the same immutable bytes. Execute
        // the ICC consistency probe once per distinct set of declarations.
        let mut colors = properties
            .iter()
            .filter(|(p, _)| p.kind == COLR)
            .map(|(p, _)| p.index)
            .collect::<Vec<_>>();
        colors.sort_unstable();
        if !checked.insert(colors) {
            continue;
        }
        if let Some(observed) = item_color_space(source, meta, id)? {
            if color.is_some_and(|c| c != observed) {
                return Err(invalid(
                    "primary image and its coded resources declare conflicting color encodings",
                ));
            }
            color = Some(observed);
        }
    }
    color.ok_or_else(|| invalid("primary image tree has no supported SDR color encoding"))
}

/// Resolve source facts before codec work for an explicitly selected profile.
pub fn primary_sdr_color_space(source: &[u8]) -> Result<xdremux_metadata::SdrColorSpace> {
    let top = scan_top_level_boxes(source)?;
    let header = one_top_level(&top.boxes, META, "meta")?;
    primary_color_space(source, &parse_meta_box(source, header)?)
}

/// Item ID that `pitm` declares as the primary image.
pub fn primary_image_item_id(source: &[u8]) -> Result<u32> {
    let top = scan_top_level_boxes(source)?;
    let header = one_top_level(&top.boxes, META, "meta")?;
    Ok(parse_meta_box(source, header)?.primary_item_id)
}

/// Raw ICC profile that describes the primary image, when the source carries one.
///
/// The profile is returned only from properties associated with the primary
/// image tree. Duplicate profiles are ambiguous and rejected.
pub fn primary_icc_profile(source: &[u8]) -> Result<Option<Vec<u8>>> {
    let top = scan_top_level_boxes(source)?;
    let header = one_top_level(&top.boxes, META, "meta")?;
    let meta = parse_meta_box(source, header)?;
    let mut indices = Vec::new();
    for id in primary_image_items(&meta)? {
        for (property, _) in item_properties(&meta, id)? {
            if property.kind == COLR && !indices.contains(&property.index) {
                indices.push(property.index);
            }
        }
    }
    let mut profile = None;
    for index in indices {
        let property = meta
            .properties
            .iter()
            .find(|p| p.index == index)
            .ok_or_else(|| invalid("primary color property is missing"))?;
        let (raw, h) = property_box(source, property)?;
        let payload = &raw[h.payload_range()];
        if matches!(payload.get(..4), Some(b"prof" | b"rICC")) {
            if profile.is_some() {
                return Err(invalid("duplicate primary ICC profile"));
            }
            profile = Some(payload[4..].to_vec());
        }
    }
    Ok(profile)
}

/// Validate the explicitly selected SDR-base / BT.2020 alternate profile.
/// Optional static metadata remains optional; absence is returned as absence.
/// Decoded pixels, dynamic tone mapping and consumer behavior are separate.
pub fn validate_hdr_gain_map_profile(
    source: &[u8],
    transfer: xdremux_metadata::HdrTransfer,
) -> Result<HdrProfileSemantics> {
    let semantics = validate_gain_map_semantics(source)?;
    if semantics.output_color
        != (HdrOutputColor::Nclx {
            primaries: 9,
            transfer: transfer.cicp(),
            matrix: 9,
            full_range: true,
        })
    {
        return Err(invalid(
            "derived HDR color must match the selected full-range BT.2020 PQ/HLG profile",
        ));
    }
    if semantics.metadata.hdr_capacity_min != 0.0 || semantics.metadata.hdr_capacity_max <= 0.0 {
        return Err(invalid(
            "BT.2020 export profiles require an SDR base and a positive alternate HDR headroom",
        ));
    }
    let top = scan_top_level_boxes(source)?;
    let header = one_top_level(&top.boxes, META, "meta")?;
    let meta = parse_meta_box(source, header)?;
    let indices = validate_meta_integrity(source, &meta)?;
    let structure = &semantics.structure;
    let ftyp = one_top_level(&top.boxes, FTYP, "ftyp")?;
    let brands = &source[ftyp.payload_range()];
    if brands.len() < 8
        || !(brands.len() - 8).is_multiple_of(4)
        || !brands[8..].as_chunks::<4>().0.contains(b"tmap")
    {
        return Err(invalid("HDR profile requires the tmap compatible brand"));
    }
    if !meta
        .iinf
        .entries
        .iter()
        .any(|i| i.item_id == structure.gain_map_item_id && i.flags & 1 != 0)
    {
        return Err(invalid("Gain Map item must be hidden"));
    }
    let base_color_space = primary_color_space(source, &meta)?;
    let orientation = |item_id| -> Result<Vec<(FourCC, u8)>> {
        let mut transforms = Vec::new();
        for (p, essential) in item_properties(&meta, item_id)? {
            if !matches!(p.kind.as_bytes(), b"irot" | b"imir") {
                continue;
            }
            let (raw, h) = property_box(source, p)?;
            let bytes = &raw[h.payload_range()];
            let limit = if p.kind.as_bytes() == b"irot" { 3 } else { 1 };
            if bytes.len() != 1
                || bytes[0] > limit
                || !essential
                || transforms.iter().any(|(k, _)| *k == p.kind)
            {
                return Err(invalid(
                    "invalid, duplicate or nonessential image orientation property",
                ));
            }
            transforms.push((p.kind, bytes[0]));
        }
        if !transforms.iter().any(|(k, _)| k.as_bytes() == b"irot") {
            return Err(invalid(
                "explicit HDR profile requires an essential irot on each image",
            ));
        }
        Ok(transforms)
    };
    let base_orientation = orientation(structure.primary_item_id)?;
    if orientation(structure.gain_map_item_id)? != base_orientation
        || orientation(structure.tmap_item_id)? != base_orientation
    {
        return Err(invalid(
            "base, Gain Map and tmap must have identical ordered orientation transforms",
        ));
    }
    let base_ispe = associated_property(
        structure.primary_item_id,
        ISPE,
        &indices.ipma_by_item,
        &indices.properties,
    )?;
    let tmap_ispe = associated_property(
        structure.tmap_item_id,
        ISPE,
        &indices.ipma_by_item,
        &indices.properties,
    )?;
    if parse_ispe(source, base_ispe)? != parse_ispe(source, tmap_ispe)? {
        return Err(invalid(
            "tmap dimensions must equal base coded dimensions before their common orientation",
        ));
    }
    let pixi = associated_property(
        structure.tmap_item_id,
        PIXI,
        &indices.ipma_by_item,
        &indices.properties,
    )?;
    if parse_pixi(source, pixi)?.1.iter().any(|b| *b < 10) {
        return Err(invalid(
            "BT.2020 HDR alternate must declare at least 10-bit channels",
        ));
    }
    let gain_color = associated_property(
        structure.gain_map_item_id,
        COLR,
        &indices.ipma_by_item,
        &indices.properties,
    )?;
    let (raw, h) = property_box(source, gain_color)?;
    if &raw[h.payload_range()] != b"nclx\0\x02\0\x02\0\x06\x80" {
        return Err(invalid(
            "unsupported Gain Map compression color encoding for the BT.2020 profile",
        ));
    }
    for id in [
        structure.primary_item_id,
        structure.gain_map_item_id,
        structure.tmap_item_id,
    ] {
        for (p, essential) in item_properties(&meta, id)? {
            if p.kind == COLR && !essential {
                return Err(invalid("profile color properties must be essential"));
            }
        }
    }
    let mut static_metadata = xdremux_metadata::HdrStaticMetadata::default();
    for (p, _) in item_properties(&meta, structure.tmap_item_id)? {
        let slot = match p.kind.as_bytes() {
            b"mdcv" => 0,
            b"clli" => 1,
            _ => continue,
        };
        let (raw, h) = property_box(source, p)?;
        let bytes = &raw[h.payload_range()];
        if slot == 0 {
            let value = xdremux_metadata::MasteringDisplay::from_mdcv_payload(bytes)
                .map_err(|e| invalid(e.to_string()))?;
            if static_metadata.mastering_display.replace(value).is_some() {
                return Err(invalid("duplicate HDR mastering property"));
            }
        } else {
            let value = xdremux_metadata::ContentLightLevel::from_clli_payload(bytes)
                .map_err(|e| invalid(e.to_string()))?;
            if static_metadata.content_light_level.replace(value).is_some() {
                return Err(invalid("duplicate HDR content light property"));
            }
        }
    }
    let mut registered_metadata = None;
    for item in meta
        .iinf
        .entries
        .iter()
        .filter(|i| i.item_type == Some(FourCC::new(*b"it35")))
    {
        if registered_metadata.is_some() {
            return Err(invalid("ambiguous registered HDR metadata items"));
        }
        let references = meta
            .iref
            .as_ref()
            .map(|r| {
                r.entries
                    .iter()
                    .filter(|r| r.kind == FourCC::new(*b"cdsc") && r.from_item_id == item.item_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if references.len() != 1 || references[0].to_item_ids != [structure.tmap_item_id] {
            return Err(invalid(
                "registered HDR metadata must describe exactly the derived HDR item",
            ));
        }
        let location = indices
            .locations
            .get(&item.item_id)
            .ok_or_else(|| invalid("registered HDR metadata has no location"))?;
        let payload = item_payload(source, meta.idat.as_ref(), location)?;
        let data = xdremux_metadata::RegisteredHdrMetadata::parse_it35(&payload)
            .map_err(|e| invalid(e.to_string()))?;
        if let Some((mastering, light)) = &data.static_metadata {
            if static_metadata.mastering_display.as_ref() != Some(mastering)
                || static_metadata.content_light_level != Some(*light)
            {
                return Err(invalid(
                    "registered HDR static metadata disagrees with mdcv/clli properties",
                ));
            }
        }
        registered_metadata = Some(data);
    }
    Ok(HdrProfileSemantics {
        semantics,
        base_color_space,
        static_metadata,
        registered_metadata,
    })
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

pub(crate) fn one_top_level<'a>(
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

fn parse_hvcc(source: &[u8], property: &PropertyInfo) -> Result<HevcDecoderConfigurationProfile> {
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
    parse_hvcc_profile(payload).map_err(|error| invalid(format!("invalid Gain Map hvcC: {error}")))
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
    let length = usize::try_from(extent_length).map_err(|_| {
        invalid(format!(
            "item {} extent length exceeds usize",
            entry.item_id
        ))
    })?;
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

pub(crate) fn item_payload(
    source: &[u8],
    idat: Option<&BoxHeader>,
    entry: &IlocEntry,
) -> Result<Vec<u8>> {
    let total = entry.extents.iter().try_fold(0usize, |total, extent| {
        let length = usize::try_from(extent.length).map_err(|_| {
            invalid(format!(
                "item {} extent length exceeds usize",
                entry.item_id
            ))
        })?;
        total
            .checked_add(length)
            .ok_or_else(|| invalid(format!("item {} payload length overflows", entry.item_id)))
    })?;
    let mut payload = Vec::with_capacity(total);
    for extent in &entry.extents {
        let range = item_extent_range(source.len(), idat, entry, extent.offset, extent.length)?;
        payload.extend_from_slice(source.get(range).ok_or_else(|| {
            invalid(format!("item {} extent is outside the HEIF", entry.item_id))
        })?);
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

pub(crate) struct MetaIndex<'a> {
    pub(crate) items: HashMap<u32, &'a xdremux_format::isobmff::ItemInfo>,
    pub(crate) locations: HashMap<u32, &'a IlocEntry>,
    pub(crate) ipma_by_item: HashMap<u32, &'a IpmaEntry>,
    pub(crate) properties: HashMap<u32, &'a PropertyInfo>,
}

pub(crate) fn validate_meta_integrity<'a>(
    source: &[u8],
    meta: &'a ParsedMeta,
) -> Result<MetaIndex<'a>> {
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

    let properties: HashMap<u32, &PropertyInfo> = meta
        .properties
        .iter()
        .map(|property| (property.index, property))
        .collect();
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

    Ok(MetaIndex {
        items,
        locations,
        ipma_by_item,
        properties,
    })
}

// Shared through structural validation with the Styles graph appender. Only tmap's
// ordered dimg image relationship identifies the Gain Map; other auxl images
// (alpha, semantic mattes, or Styles resources) never become candidates.
fn gain_map_graph_roots(meta: &ParsedMeta) -> Result<(u32, u32)> {
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
    if gain_map_item_id == meta.primary_item_id
        || gain_map_item_id == tmap_item_id
        || tmap_item_id == meta.primary_item_id
    {
        return Err(invalid(
            "primary, tmap, and Gain Map roles must use distinct item IDs",
        ));
    }
    let gain_items = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_id == gain_map_item_id)
        .collect::<Vec<_>>();
    match gain_items.as_slice() {
        [item] if matches!(item.item_type, Some(GRID) | Some(HVC1)) => {}
        [_] => return Err(invalid("Gain Map image must be grid or hvc1")),
        [] => {
            return Err(invalid(format!(
                "gain-map item {gain_map_item_id} is missing"
            )))
        }
        _ => return Err(invalid("Gain Map item ID is ambiguous")),
    }
    Ok((tmap_item_id, gain_map_item_id))
}

/// Validate ISO-BMFF/HEVC Gain Map structure for direct hvc1 or tiled grid images.
/// This does not claim Apple consumer recognition; ImageIO/Photos validation
/// remains an Apple-platform gate.
pub fn validate_gain_map_structure(source: &[u8]) -> Result<GainMapStructure> {
    let top = scan_top_level_boxes(source)?;
    let _ftyp = one_top_level(&top.boxes, FTYP, "ftyp")?;
    let meta_header = one_top_level(&top.boxes, META, "meta")?;
    let _mdat = one_top_level(&top.boxes, MDAT, "mdat")?;
    let meta = parse_meta_box(source, meta_header)?;
    let MetaIndex {
        items,
        locations,
        ipma_by_item,
        properties,
    } = validate_meta_integrity(source, &meta)?;

    let (tmap_item_id, gain_map_item_id) = gain_map_graph_roots(&meta)?;
    let iref = meta
        .iref
        .as_ref()
        .ok_or_else(|| invalid("tmap graph requires iref"))?;
    let gain_map_item = items
        .get(&gain_map_item_id)
        .copied()
        .ok_or_else(|| invalid(format!("gain-map item {gain_map_item_id} is missing")))?;
    let gain_refs: Vec<_> = iref
        .entries
        .iter()
        .filter(|reference| reference.kind == DIMG && reference.from_item_id == gain_map_item_id)
        .collect();
    let gain_location = locations.get(&gain_map_item_id).copied().ok_or_else(|| {
        invalid(format!(
            "gain-map item {gain_map_item_id} has no iloc entry"
        ))
    })?;
    let gain_ispe = associated_property(gain_map_item_id, ISPE, &ipma_by_item, &properties)?;
    let (ispe_width, ispe_height) = parse_ispe(source, gain_ispe)?;
    let (tile_item_ids, rows, columns) = match gain_map_item.item_type {
        Some(GRID) => {
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
            if (ispe_width, ispe_height) != (grid.width, grid.height) {
                return Err(invalid(format!(
                    "grid payload dimensions {}x{} disagree with ispe {}x{}",
                    grid.width, grid.height, ispe_width, ispe_height
                )));
            }
            (tile_item_ids, grid.rows, grid.columns)
        }
        Some(HVC1) => {
            if !gain_refs.is_empty() {
                return Err(invalid(
                    "direct hvc1 Gain Map must not have a dimg reference",
                ));
            }
            (vec![gain_map_item_id], 1, 1)
        }
        _ => return Err(invalid("Gain Map image must be grid or hvc1")),
    };
    if let Some(geometry) = crate::orientation::coded_geometry_if_present(source, &meta)? {
        crate::orientation::require_coded_aspect(
            geometry.width,
            geometry.height,
            ispe_width,
            ispe_height,
            "Gain Map",
        )?;
    }
    let gain_pixi = associated_property(gain_map_item_id, PIXI, &ipma_by_item, &properties)?;
    let (channel_count, channel_bits) = parse_pixi(source, gain_pixi)?;
    if !matches!(channel_count, 1 | 3) {
        return Err(invalid(format!(
            "gain-map pixi must declare one or three channels, got {channel_count}"
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

    let mut expected_codec: Option<HevcDecoderConfigurationProfile> = None;
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
        if *tile_item_id == meta.primary_item_id
            || iref.entries.iter().any(|reference| {
                reference.kind == DIMG
                    && reference.from_item_id == meta.primary_item_id
                    && reference.to_item_ids.contains(tile_item_id)
            })
        {
            return Err(invalid(
                "primary and Gain Map cannot share a coded resource",
            ));
        }
        if iref
            .entries
            .iter()
            .any(|reference| reference.kind == DIMG && reference.from_item_id == *tile_item_id)
        {
            return Err(invalid(format!(
                "coded hvc1 item {tile_item_id} must not have a dimg reference"
            )));
        }
        let location = locations
            .get(tile_item_id)
            .copied()
            .ok_or_else(|| invalid(format!("tile item {tile_item_id} has no iloc entry")))?;
        // validate_meta_integrity has checked every extent against its actual
        // backing bytes. Require nonempty coded data without copying payloads.
        if location.extents.iter().all(|extent| extent.length == 0) {
            return Err(invalid(format!(
                "coded hvc1 item {tile_item_id} has an empty payload"
            )));
        }
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
    let codec_channels = codec.chroma_sampling.semantic_channel_count();
    if channel_count != codec_channels {
        return Err(invalid(format!(
            "gain-map pixi declares {channel_count} channels but hvcC {:?} implies {codec_channels}",
            codec.chroma_sampling
        )));
    }
    let expected_channel_bits = match channel_count {
        1 => vec![codec.luma_bit_depth],
        3 => vec![
            codec.luma_bit_depth,
            codec.chroma_bit_depth,
            codec.chroma_bit_depth,
        ],
        _ => unreachable!("channel count validated above"),
    };
    if channel_bits != expected_channel_bits {
        return Err(invalid(format!(
            "gain-map pixi bit depths {:?} disagree with hvcC {}/{}",
            channel_bits, codec.luma_bit_depth, codec.chroma_bit_depth
        )));
    }

    Ok(GainMapStructure {
        primary_item_id: meta.primary_item_id,
        tmap_item_id,
        gain_map_item_id,
        tile_item_ids,
        width: ispe_width,
        height: ispe_height,
        rows,
        columns,
        channel_count,
        chroma_format_idc: codec.chroma_sampling.hevc_chroma_format_idc(),
        bit_depth: codec.luma_bit_depth,
        chroma_sampling: codec.chroma_sampling,
        luma_bit_depth: codec.luma_bit_depth,
        chroma_bit_depth: codec.chroma_bit_depth,
        general_profile_idc: codec.general_profile_idc,
    })
}

fn read_hdr_output_color(source: &[u8], property: &PropertyInfo) -> Result<HdrOutputColor> {
    let (raw, header) = property_box(source, property)?;
    let payload = raw
        .get(header.payload_range())
        .ok_or_else(|| invalid("HDR colr payload is outside its property box"))?;
    match payload.get(..4) {
        Some(four) if four == b"nclx" && payload.len() == 11 => {
            let read = |offset| u16::from_be_bytes([payload[offset], payload[offset + 1]]);
            let (primaries, transfer, matrix) = (read(4), read(6), read(8));
            if primaries == 0 || primaries == 2 || transfer == 0 || transfer == 2 {
                return Err(invalid("HDR output nclx does not identify its colorimetry"));
            }
            if payload[10] & 0x7f != 0 {
                return Err(invalid("HDR output nclx has nonzero reserved bits"));
            }
            Ok(HdrOutputColor::Nclx {
                primaries,
                transfer,
                matrix,
                full_range: payload[10] & 0x80 != 0,
            })
        }
        Some(four) if four == b"prof" || four == b"rICC" => {
            xdremux_metadata::validate_iso_alternate_icc(&payload[4..], true)
                .map_err(|error| invalid(format!("HDR output ICC: {error}")))?;
            Ok(HdrOutputColor::Icc)
        }
        _ => Err(invalid(
            "HDR output colr must contain a valid nclx or ICC profile",
        )),
    }
}

/// Validate gain-map numeric semantics and output color properties in addition
/// to the structural graph. Does not infer application primaries from the
/// Gain Map's compression color space or equate metadata and pixel channels.
pub fn validate_gain_map_semantics(source: &[u8]) -> Result<GainMapSemantics> {
    let structure = validate_gain_map_structure(source)?;
    let top = scan_top_level_boxes(source)?;
    let meta_header = one_top_level(&top.boxes, META, "meta")?;
    let meta = parse_meta_box(source, meta_header)?;
    let indices = validate_meta_integrity(source, &meta)?;

    let location = indices
        .locations
        .get(&structure.tmap_item_id)
        .ok_or_else(|| invalid("tmap has no iloc entry"))?;
    let payload = item_payload(source, meta.idat.as_ref(), location)?;
    let metadata = xdremux_metadata::parse_heif_tmap_metadata(&payload)
        .map_err(|error| invalid(format!("invalid tmap metadata: {error}")))?;
    if metadata.channel_count == 3 && structure.channel_count != 3 {
        return Err(invalid(
            "three-component tmap metadata requires an RGB Gain Map",
        ));
    }

    let tmap_pixi = associated_property(
        structure.tmap_item_id,
        PIXI,
        &indices.ipma_by_item,
        &indices.properties,
    )?;
    let (channels, bits) = parse_pixi(source, tmap_pixi)?;
    if channels != 3 || bits.contains(&0) {
        return Err(invalid(
            "derived HDR tmap pixi must describe three nonzero-depth channels",
        ));
    }

    let color = associated_property(
        structure.tmap_item_id,
        COLR,
        &indices.ipma_by_item,
        &indices.properties,
    )?;
    let output_color = read_hdr_output_color(source, color)?;
    Ok(GainMapSemantics {
        structure,
        metadata,
        output_color,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{
        make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
        make_iref_box, make_ispe_box, make_pitm_box, IlocExtent, IpmaAssociation, IrefEntry, IDAT,
        IPCO, IPRP,
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
        gain_pixi_bit_depth: u8,
        hvcc_luma_bit_depth: u8,
        hvcc_chroma_bit_depth: u8,
        tile_extent_length: u64,
        grid_rows: u8,
        grid_columns: u8,
        gain_item_type: FourCC,
        primary_grid: bool,
        gain_ispe_dimensions: (u32, u32),
        gain_hvcc_count: usize,
        gain_omit_property: Option<FourCC>,
        gain_payload_length: Option<u64>,
        extra_references: Vec<IrefEntry>,
        tmap_payload: Vec<u8>,
        tmap_color: Vec<u8>,
    }

    impl Default for FixtureOptions {
        fn default() -> Self {
            Self {
                tmap_targets: vec![PRIMARY_ID, GAIN_ID],
                gain_targets: vec![TILE_ID],
                gain_pixi_channels: 3,
                hvcc_chroma: 3,
                gain_pixi_bit_depth: 8,
                hvcc_luma_bit_depth: 8,
                hvcc_chroma_bit_depth: 8,
                tile_extent_length: 1,
                grid_rows: 1,
                grid_columns: 1,
                gain_item_type: GRID,
                primary_grid: false,
                gain_ispe_dimensions: (4, 4),
                gain_hvcc_count: 1,
                gain_omit_property: None,
                gain_payload_length: None,
                extra_references: Vec::new(),
                tmap_payload: vec![0xb2],
                tmap_color: b"nclx\0\x09\0\x10\0\x09\x80".to_vec(),
            }
        }
    }

    fn pixi_box(channels: u8, bit_depth: u8) -> Vec<u8> {
        let mut payload = vec![channels];
        payload.extend(std::iter::repeat_n(bit_depth, usize::from(channels)));
        make_full_box(PIXI, 0, 0, &payload).unwrap()
    }

    fn hvcc_box(chroma: u8, luma_bit_depth: u8, chroma_bit_depth: u8) -> Vec<u8> {
        let mut payload = vec![0u8; 19];
        payload[0] = 1;
        payload[1] = 4;
        payload[16] = chroma;
        payload[17] = luma_bit_depth - 8;
        payload[18] = chroma_bit_depth - 8;
        make_box(HVCC, &payload).unwrap()
    }

    fn fixture(options: FixtureOptions) -> Vec<u8> {
        let mut grid_payload = vec![0, 0, options.grid_rows - 1, options.grid_columns - 1];
        grid_payload.extend_from_slice(&4u16.to_be_bytes());
        grid_payload.extend_from_slice(&4u16.to_be_bytes());
        let primary_payload = if options.primary_grid {
            grid_payload.clone()
        } else {
            vec![0xa1]
        };
        let tmap_payload = options.tmap_payload.as_slice();
        let tile_payload = [0xc3];
        let mut idat_payload = Vec::new();
        let primary_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&primary_payload);
        let gain_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&grid_payload);
        let tmap_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(tmap_payload);
        let tile_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(&tile_payload);

        let gain_ispe = make_ispe_box(
            options.gain_ispe_dimensions.0,
            options.gain_ispe_dimensions.1,
        )
        .unwrap();
        let gain_pixi = pixi_box(options.gain_pixi_channels, options.gain_pixi_bit_depth);
        let tmap_ispe = make_ispe_box(4, 4).unwrap();
        let tmap_pixi = pixi_box(3, 8);
        let tile_ispe = make_ispe_box(4, 4).unwrap();
        let tile_hvcc = hvcc_box(
            options.hvcc_chroma,
            options.hvcc_luma_bit_depth,
            options.hvcc_chroma_bit_depth,
        );
        let mut ipco_payload = Vec::new();
        for property in [
            &gain_ispe,
            &gain_pixi,
            &tmap_ispe,
            &tmap_pixi,
            &tile_ispe,
            &tile_hvcc,
            &tile_hvcc,
            &make_ispe_box(4, 4).unwrap(),
            &make_box(COLR, &options.tmap_color).unwrap(),
        ] {
            ipco_payload.extend_from_slice(property);
        }
        let ipco = make_box(IPCO, &ipco_payload).unwrap();
        let mut gain_associations = [(1, ISPE), (2, PIXI)]
            .into_iter()
            .filter(|(_, kind)| Some(*kind) != options.gain_omit_property)
            .map(|(property_index, _)| IpmaAssociation {
                property_index,
                essential: false,
            })
            .collect::<Vec<_>>();
        if options.gain_item_type == HVC1 && options.gain_omit_property != Some(HVCC) {
            gain_associations.extend((0..options.gain_hvcc_count).map(|index| IpmaAssociation {
                property_index: u16::try_from(6 + index).unwrap(),
                essential: true,
            }));
        }
        let ipma = make_ipma_box(
            0,
            0,
            &[
                IpmaEntry {
                    item_id: PRIMARY_ID,
                    associations: vec![IpmaAssociation {
                        property_index: 8,
                        essential: true,
                    }],
                },
                IpmaEntry {
                    item_id: GAIN_ID,
                    associations: gain_associations,
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
                        IpmaAssociation {
                            property_index: 9,
                            essential: true,
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
            make_infe_box(
                PRIMARY_ID,
                if options.primary_grid { GRID } else { HVC1 },
                0,
            )
            .unwrap(),
            make_infe_box(GAIN_ID, options.gain_item_type, 1).unwrap(),
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
                        length: primary_payload.len() as u64,
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
                        length: options
                            .gain_payload_length
                            .unwrap_or(grid_payload.len() as u64),
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
                        length: tmap_payload.len() as u64,
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
        let mut references = vec![IrefEntry {
            kind: DIMG,
            from_item_id: TMAP_ID,
            to_item_ids: options.tmap_targets,
        }];
        if options.gain_item_type == GRID || !options.gain_targets.is_empty() {
            references.push(IrefEntry {
                kind: DIMG,
                from_item_id: GAIN_ID,
                to_item_ids: options.gain_targets,
            });
        }
        references.extend(options.extra_references);
        let iref = make_iref_box(0, &references).unwrap();
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

    fn direct_gain_options() -> FixtureOptions {
        FixtureOptions {
            gain_item_type: HVC1,
            gain_targets: Vec::new(),
            ..FixtureOptions::default()
        }
    }

    fn valid_tmap_payload() -> Vec<u8> {
        let mut bytes = vec![0, 0, 0, 0, 0, 0x40];
        for (numerator, denominator) in [(0_u32, 1_u32), (2, 1)] {
            bytes.extend_from_slice(&numerator.to_be_bytes());
            bytes.extend_from_slice(&denominator.to_be_bytes());
        }
        for (numerator, denominator) in [(0_i32, 1_u32), (2, 1)] {
            bytes.extend_from_slice(&numerator.to_be_bytes());
            bytes.extend_from_slice(&denominator.to_be_bytes());
        }
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        for _ in 0..2 {
            bytes.extend_from_slice(&0_i32.to_be_bytes());
            bytes.extend_from_slice(&1_u32.to_be_bytes());
        }
        assert_eq!(bytes.len(), 62);
        bytes
    }

    #[test]
    fn semantic_validation_reads_real_tmap_and_independent_channel_counts() {
        let source = fixture(FixtureOptions {
            tmap_payload: valid_tmap_payload(),
            ..FixtureOptions::default()
        });
        let result = validate_gain_map_semantics(&source).unwrap();
        assert_eq!(result.structure.channel_count, 3);
        assert_eq!(result.metadata.channel_count, 1);
        assert_eq!(result.metadata.hdr_capacity_max, 2.0);
        assert!(result.metadata.use_base_color_space);
        assert_eq!(
            result.output_color,
            HdrOutputColor::Nclx {
                primaries: 9,
                transfer: 16,
                matrix: 9,
                full_range: true,
            }
        );
    }

    #[test]
    fn semantic_validation_rejects_invalid_payload_without_changing_structural_contract() {
        for (case, mutate, expected) in [
            ("version", 0, "tone-map version"),
            ("reserved flags", 5, "reserved gain-map flags"),
            ("zero denominator", 10, "zero denominator"),
        ] {
            let mut payload = valid_tmap_payload();
            payload[mutate] = if case == "zero denominator" { 0 } else { 1 };
            // The denominator's last byte is at offset 13.
            if case == "zero denominator" {
                payload[13] = 0;
            }
            let source = fixture(FixtureOptions {
                tmap_payload: payload,
                ..FixtureOptions::default()
            });
            validate_gain_map_structure(&source).unwrap();
            let error = validate_gain_map_semantics(&source)
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{case}: {error}");
        }
    }

    #[test]
    fn semantic_validation_rejects_ambiguous_hdr_colorimetry() {
        let source = fixture(FixtureOptions {
            tmap_payload: valid_tmap_payload(),
            tmap_color: b"nclx\0\x02\0\x02\0\x09\x80".to_vec(),
            ..FixtureOptions::default()
        });
        validate_gain_map_structure(&source).unwrap();
        assert!(validate_gain_map_semantics(&source)
            .unwrap_err()
            .to_string()
            .contains("does not identify its colorimetry"));
    }

    #[test]
    fn semantic_validation_rejects_rgb_metadata_for_a_monochrome_gain_map() {
        let mono = valid_tmap_payload();
        let mut rgb = mono[..22].to_vec();
        rgb[5] |= 0x80;
        for _ in 0..3 {
            rgb.extend_from_slice(&mono[22..]);
        }
        let source = fixture(FixtureOptions {
            tmap_payload: rgb,
            gain_pixi_channels: 1,
            hvcc_chroma: 0,
            ..FixtureOptions::default()
        });
        validate_gain_map_structure(&source).unwrap();
        assert!(validate_gain_map_semantics(&source).is_err());
    }

    #[test]
    fn direct_gain_map_reports_its_actual_coded_resource_and_precision() {
        for (chroma, channels, depth) in [(0, 1, 8), (1, 3, 8), (2, 3, 10), (3, 3, 10)] {
            let source = fixture(FixtureOptions {
                gain_pixi_channels: channels,
                hvcc_chroma: chroma,
                gain_pixi_bit_depth: depth,
                hvcc_luma_bit_depth: depth,
                hvcc_chroma_bit_depth: depth,
                ..direct_gain_options()
            });
            let actual = validate_gain_map_structure(&source).unwrap();
            assert_eq!(actual.gain_map_item_id, GAIN_ID);
            assert_eq!(actual.tile_item_ids, vec![GAIN_ID]);
            assert_eq!(
                (actual.width, actual.height, actual.rows, actual.columns),
                (4, 4, 1, 1)
            );
            assert_eq!(actual.channel_count, channels);
            assert_eq!(actual.chroma_format_idc, chroma);
            assert_eq!(
                (actual.luma_bit_depth, actual.chroma_bit_depth),
                (depth, depth)
            );
        }
    }

    #[test]
    fn direct_gain_map_rejects_incomplete_ambiguous_or_conflicting_resources() {
        for case in [
            "derived",
            "empty derived",
            "missing hvcC",
            "duplicate hvcC",
            "missing pixi",
            "missing ispe",
            "empty payload",
            "outside payload",
            "channels",
            "precision",
            "geometry",
            "primary alias",
            "primary coded alias",
            "tmap alias",
            "unknown type",
            "unknown reference",
            "duplicate tmap",
        ] {
            let mut options = direct_gain_options();
            match case {
                "derived" => options.gain_targets = vec![TILE_ID],
                "empty derived" => options.extra_references.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: GAIN_ID,
                    to_item_ids: Vec::new(),
                }),
                "missing hvcC" => options.gain_omit_property = Some(HVCC),
                "duplicate hvcC" => options.gain_hvcc_count = 2,
                "missing pixi" => options.gain_omit_property = Some(PIXI),
                "missing ispe" => options.gain_omit_property = Some(ISPE),
                "empty payload" => options.gain_payload_length = Some(0),
                "outside payload" => options.gain_payload_length = Some(9999),
                "channels" => options.hvcc_chroma = 0,
                "precision" => options.gain_pixi_bit_depth = 10,
                "geometry" => options.gain_ispe_dimensions = (4, 2),
                "primary alias" => options.tmap_targets = vec![PRIMARY_ID, PRIMARY_ID],
                "primary coded alias" => {
                    options.primary_grid = true;
                    options.extra_references.push(IrefEntry {
                        kind: DIMG,
                        from_item_id: PRIMARY_ID,
                        to_item_ids: vec![GAIN_ID],
                    });
                }
                "tmap alias" => options.tmap_targets = vec![PRIMARY_ID, TMAP_ID],
                "unknown type" => options.gain_item_type = FourCC::new(*b"jpeg"),
                "unknown reference" => options.extra_references.push(IrefEntry {
                    kind: FourCC::new(*b"auxl"),
                    from_item_id: GAIN_ID,
                    to_item_ids: vec![99],
                }),
                "duplicate tmap" => options.extra_references.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: TMAP_ID,
                    to_item_ids: vec![PRIMARY_ID, GAIN_ID],
                }),
                _ => unreachable!(),
            }
            let expected = match case {
                "derived" | "empty derived" => "must not have a dimg reference",
                "missing hvcC" => "missing hvcC",
                "duplicate hvcC" => "more than one hvcC",
                "missing pixi" => "missing pixi",
                "missing ispe" => "missing ispe",
                "empty payload" => "empty payload",
                "outside payload" => "exceeds its backing storage",
                "channels" => "implies 1",
                "precision" => "disagree with hvcC",
                "geometry" => "does not match primary coded orientation",
                "primary alias" | "tmap alias" => "distinct item IDs",
                "primary coded alias" => "cannot share a coded resource",
                "unknown type" => "must be grid or hvc1",
                "unknown reference" => "targets unknown item 99",
                "duplicate tmap" => "exactly one dimg reference",
                _ => unreachable!(),
            };
            let error = validate_gain_map_structure(&fixture(options))
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{case}: {error}");
        }
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
    #[test]
    fn accepts_mono_420_422_444_and_ten_bit_profiles() {
        for (hvcc_chroma, channels) in [(0, 1), (1, 3), (2, 3), (3, 3)] {
            let options = FixtureOptions {
                gain_pixi_channels: channels,
                hvcc_chroma,
                gain_pixi_bit_depth: 10,
                hvcc_luma_bit_depth: 10,
                hvcc_chroma_bit_depth: 10,
                ..FixtureOptions::default()
            };
            let result = validate_gain_map_structure(&fixture(options)).unwrap();
            assert_eq!(result.channel_count, channels);
            assert_eq!(result.chroma_format_idc, hvcc_chroma);
            assert_eq!(result.luma_bit_depth, 10);
            assert_eq!(result.chroma_bit_depth, 10);
        }
    }
}

use std::collections::BTreeSet;

use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
    make_iref_box, make_ispe_box, parse_boxes, parse_meta_box, scan_top_level_boxes, BoxHeader,
    IlocEntry, IlocExtent, IpmaAssociation, IpmaEntry, IrefEntry, ParsedMeta, PropertyInfo, FTYP,
    IDAT, IINF, ILOC, IPCO, IPMA, IPRP, IREF, MDAT, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const GRID: FourCC = FourCC::new(*b"grid");
const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const TMAP: FourCC = FourCC::new(*b"tmap");
const URI: FourCC = FourCC::new(*b"uri ");
const IROT: FourCC = FourCC::new(*b"irot");
const COLR: FourCC = FourCC::new(*b"colr");
const PIXI: FourCC = FourCC::new(*b"pixi");
const AUXC: FourCC = FourCC::new(*b"auxC");
const DIMG: FourCC = FourCC::new(*b"dimg");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const HDLR: FourCC = FourCC::new(*b"hdlr");
const DINF: FourCC = FourCC::new(*b"dinf");
const PITM: FourCC = FourCC::new(*b"pitm");
const GRPL: FourCC = FourCC::new(*b"grpl");

const STYLE_METADATA_URI: &[u8] = b"tag:apple.com,2023:photo:metadata:styles";
const STYLE_DELTA_AUX_TYPE: &[u8] = b"tag:apple.com,2023:photo:aux:styledeltamap";
const LINEAR_THUMBNAIL_AUX_TYPE: &[u8] = b"tag:apple.com,2023:photo:aux:linearthumbnail";
const STYLE_DELTA_TILE_COUNT: u32 = 30;

/// Rust-owned inputs for the incremental Photographic Styles HEIF graph.
///
/// The style solver, metadata policy, and codec producers live above this
/// type. This layer only assembles their already-produced resources into the
/// ISO-BMFF item graph. The source base/Gain Map mdat prefix is preserved
/// byte-for-byte and all existing file-backed iloc extents are relocated in a
/// checked two-pass rewrite.
#[derive(Debug, Clone, Copy)]
pub struct PhotographicStylesAssembly<'a> {
    pub style_property_list: &'a [u8],
    pub style_delta_hvcc: &'a [u8],
    pub style_delta_tile_payload: &'a [u8],
    pub style_delta_tile_width: u32,
    pub style_delta_tile_height: u32,
    pub style_delta_grid_width: u32,
    pub style_delta_grid_height: u32,
    pub style_delta_rows: u32,
    pub style_delta_columns: u32,
    pub linear_thumbnail_hvcc: &'a [u8],
    pub linear_thumbnail_payload: &'a [u8],
    pub linear_thumbnail_width: u32,
    pub linear_thumbnail_height: u32,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
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

fn one_top_level<'a>(boxes: &'a [BoxHeader], kind: FourCC, context: &str) -> Result<&'a BoxHeader> {
    let mut matches = boxes.iter().filter(|header| header.kind == kind);
    let Some(first) = matches.next() else {
        return Err(invalid(format!("{context} is missing")));
    };
    if matches.next().is_some() {
        return Err(invalid(format!("{context} appears more than once")));
    }
    Ok(first)
}

fn child<'a>(children: &'a [BoxHeader], kind: FourCC, context: &str) -> Result<&'a BoxHeader> {
    children
        .iter()
        .find(|header| header.kind == kind)
        .ok_or_else(|| invalid(format!("{context}/{} is missing", kind)))
}

fn validate_assembly(assembly: &PhotographicStylesAssembly<'_>) -> Result<()> {
    if assembly.style_property_list.len() < 8
        || !assembly.style_property_list.starts_with(b"bplist00")
    {
        return Err(invalid(
            "Photographic Styles metadata must be a binary property list",
        ));
    }
    if assembly.style_delta_hvcc.is_empty() || assembly.style_delta_tile_payload.is_empty() {
        return Err(invalid(
            "Photographic Styles Style Delta requires hvcC and tile payloads",
        ));
    }
    if assembly.linear_thumbnail_hvcc.is_empty() || assembly.linear_thumbnail_payload.is_empty() {
        return Err(invalid(
            "Photographic Styles Linear Thumbnail requires hvcC and payload",
        ));
    }
    for (value, context) in [
        (assembly.style_delta_tile_width, "Style Delta tile width"),
        (assembly.style_delta_tile_height, "Style Delta tile height"),
        (assembly.style_delta_grid_width, "Style Delta grid width"),
        (assembly.style_delta_grid_height, "Style Delta grid height"),
        (assembly.linear_thumbnail_width, "Linear Thumbnail width"),
        (assembly.linear_thumbnail_height, "Linear Thumbnail height"),
    ] {
        if value == 0 {
            return Err(invalid(format!("{context} must be non-zero")));
        }
    }
    for (value, context) in [
        (assembly.style_delta_rows, "Style Delta rows"),
        (assembly.style_delta_columns, "Style Delta columns"),
    ] {
        if !(1..=256).contains(&value) {
            return Err(invalid(format!("{context} must be within 1 through 256")));
        }
    }
    let tile_count = assembly
        .style_delta_rows
        .checked_mul(assembly.style_delta_columns)
        .ok_or_else(|| invalid("Style Delta tile count overflows"))?;
    if tile_count != STYLE_DELTA_TILE_COUNT {
        return Err(invalid(format!(
            "Style Delta grid must contain {STYLE_DELTA_TILE_COUNT} tiles, got {tile_count}"
        )));
    }
    Ok(())
}

fn find_style_metadata_item(source: &[u8], meta: &ParsedMeta) -> Result<()> {
    for item in &meta.iinf.entries {
        if item.item_type != Some(URI) {
            continue;
        }
        let raw = source
            .get(item.box_range.clone())
            .ok_or_else(|| invalid(format!("uri item {} is outside source", item.item_id)))?;
        if raw
            .windows(STYLE_METADATA_URI.len())
            .any(|window| window == STYLE_METADATA_URI)
        {
            return Err(invalid(
                "source already contains a Photographic Styles metadata item",
            ));
        }
    }
    Ok(())
}

fn find_style_graph_roots(meta: &ParsedMeta) -> Result<(u32, u32)> {
    let tmap_ids = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(TMAP))
        .map(|item| item.item_id)
        .collect::<Vec<_>>();
    let tmap_id = match tmap_ids.as_slice() {
        [only] => *only,
        [] => return Err(invalid("Photographic Styles source has no tmap item")),
        _ => {
            return Err(invalid(
                "Photographic Styles source has multiple tmap items",
            ))
        }
    };
    let Some(iref) = meta.iref.as_ref() else {
        return Err(invalid(
            "Photographic Styles source has no iref graph for its Gain Map",
        ));
    };
    let mut candidates = BTreeSet::new();
    for reference in &iref.entries {
        if reference.kind == DIMG && reference.from_item_id == tmap_id {
            for item_id in &reference.to_item_ids {
                if *item_id != meta.primary_item_id {
                    candidates.insert(*item_id);
                }
            }
        }
        if reference.kind == AUXL
            && reference.to_item_ids.contains(&meta.primary_item_id)
            && reference.to_item_ids.contains(&tmap_id)
        {
            candidates.insert(reference.from_item_id);
        }
    }
    // ImageIO may add an auxiliary item that also references the primary and
    // tmap roots (for example, a Vision sky matte). The Gain Map item is the
    // grid among those candidates; accepting every auxl source would make a
    // valid source ambiguous as soon as a semantic resource is present.
    let gain_grid_candidates = candidates
        .into_iter()
        .filter(|item_id| {
            meta.iinf
                .entries
                .iter()
                .any(|item| item.item_id == *item_id && item.item_type == Some(GRID))
        })
        .collect::<Vec<_>>();
    let gain_id = match gain_grid_candidates.as_slice() {
        [only] => *only,
        [] => {
            return Err(invalid(
                "Photographic Styles source has no unambiguous Gain Map grid item",
            ));
        }
        _ => {
            return Err(invalid(
                "Photographic Styles source has multiple Gain Map grid candidates",
            ));
        }
    };
    Ok((tmap_id, gain_id))
}

fn associated_property_index(meta: &ParsedMeta, item_id: u32, kind: FourCC) -> Option<u16> {
    let entry = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)?;
    entry.associations.iter().find_map(|association| {
        meta.properties
            .iter()
            .find(|property| property.index == u32::from(association.property_index))
            .filter(|property| property.kind == kind)
            .map(|_| association.property_index)
    })
}

fn style_delta_color_property_index(meta: &ParsedMeta) -> Result<u16> {
    if let Some(index) = associated_property_index(meta, meta.primary_item_id, COLR) {
        return Ok(index);
    }

    // A Rust-authored ISO Gain Map base can leave the primary grid with only
    // its geometry association. Its primary color profile is still carried by
    // the grid's hvc1 children. Reuse that exact source property for the Style
    // Delta resources; this preserves the producer's color contract without
    // inventing a platform-specific profile in the container layer.
    let mut candidates = BTreeSet::new();
    if let Some(iref) = meta.iref.as_ref() {
        for reference in &iref.entries {
            if reference.kind != DIMG || reference.from_item_id != meta.primary_item_id {
                continue;
            }
            for item_id in &reference.to_item_ids {
                let is_primary_component = meta
                    .iinf
                    .entries
                    .iter()
                    .any(|item| item.item_id == *item_id && item.item_type == Some(HVC1));
                if !is_primary_component {
                    continue;
                }
                if let Some(index) = associated_property_index(meta, *item_id, COLR) {
                    candidates.insert(index);
                }
            }
        }
    }
    match candidates.len() {
        1 => candidates
            .into_iter()
            .next()
            .ok_or_else(|| invalid("Style Delta source color property disappeared")),
        0 => Err(invalid(
            "Photographic Styles source has no verifiable primary color property",
        )),
        _ => Err(invalid(
            "Photographic Styles source has ambiguous primary color properties",
        )),
    }
}

fn append_property(output: &mut Vec<u8>, next_index: &mut u16, raw: &[u8]) -> Result<u16> {
    if *next_index == 0 || *next_index > 0x7fff {
        return Err(invalid(
            "Photographic Styles property index exceeds 15 bits",
        ));
    }
    let index = *next_index;
    output.extend_from_slice(raw);
    *next_index = next_index
        .checked_add(1)
        .ok_or_else(|| invalid("Photographic Styles property index overflows"))?;
    Ok(index)
}

fn make_auxc_box(auxiliary_type: &[u8]) -> Result<Vec<u8>> {
    let mut payload = auxiliary_type.to_vec();
    payload.push(0);
    Ok(make_full_box(AUXC, 0, 0, &payload)?)
}

fn make_style_pixi_box() -> Result<Vec<u8>> {
    Ok(make_full_box(PIXI, 0, 0, &[3, 10, 10, 10])?)
}

fn make_uri_infe_box(item_id: u32) -> Result<Vec<u8>> {
    let version = if item_id <= u32::from(u16::MAX) { 2 } else { 3 };
    let mut payload = Vec::new();
    if version == 2 {
        payload.extend_from_slice(&(item_id as u16).to_be_bytes());
    } else {
        payload.extend_from_slice(&item_id.to_be_bytes());
    }
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(URI.as_bytes());
    payload.extend_from_slice(b"styleMetadata");
    payload.push(0);
    payload.extend_from_slice(STYLE_METADATA_URI);
    payload.push(0);
    Ok(make_full_box(FourCC::new(*b"infe"), version, 1, &payload)?)
}

fn make_grid_payload(rows: u32, columns: u32, width: u32, height: u32) -> Result<Vec<u8>> {
    if !(1..=256).contains(&rows) || !(1..=256).contains(&columns) || width == 0 || height == 0 {
        return Err(invalid("invalid Photographic Styles grid geometry"));
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

fn build_iprp(
    source: &[u8],
    iprp_header: &BoxHeader,
    meta: &ParsedMeta,
    tile_ids: &[u32],
    delta_grid_id: u32,
    linear_thumbnail_id: u32,
    assembly: &PhotographicStylesAssembly<'_>,
) -> Result<Vec<u8>> {
    let mut properties = meta.properties.iter().collect::<Vec<_>>();
    properties.sort_by_key(|property| property.index);
    for (offset, property) in properties.iter().enumerate() {
        let expected = u32::try_from(offset + 1)
            .map_err(|_| invalid("source HEIF property count exceeds u32"))?;
        if property.index != expected {
            return Err(invalid("source ipco property indices are not contiguous"));
        }
    }
    let mut ipco_payload = Vec::new();
    for property in &properties {
        ipco_payload.extend_from_slice(raw_property(source, property)?);
    }
    let mut next_property_index = u16::try_from(properties.len() + 1)
        .map_err(|_| invalid("source HEIF property count exceeds u16"))?;

    let primary_color_index = style_delta_color_property_index(meta)?;
    let delta_hvcc_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_box(HVCC, assembly.style_delta_hvcc)?,
    )?;
    let delta_tile_ispe_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_ispe_box(
            assembly.style_delta_tile_width,
            assembly.style_delta_tile_height,
        )?,
    )?;
    let style_pixi_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_style_pixi_box()?,
    )?;
    let delta_grid_ispe_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_ispe_box(
            assembly.style_delta_grid_width,
            assembly.style_delta_grid_height,
        )?,
    )?;
    let delta_auxc_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_auxc_box(STYLE_DELTA_AUX_TYPE)?,
    )?;
    let identity_irot_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        // `irot` is a one-byte item property, not a FullBox.  Apple native
        // Styles files use the ISO-BMFF wire form `size + "irot" + angle`;
        // adding version/flags here changes the property payload to five
        // bytes and makes ImageIO reject the otherwise valid Styles graph.
        &make_box(IROT, &[0])?,
    )?;
    let linear_hvcc_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_box(HVCC, assembly.linear_thumbnail_hvcc)?,
    )?;
    let linear_ispe_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_ispe_box(
            assembly.linear_thumbnail_width,
            assembly.linear_thumbnail_height,
        )?,
    )?;
    let linear_auxc_index = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_auxc_box(LINEAR_THUMBNAIL_AUX_TYPE)?,
    )?;

    let ipco = make_box(IPCO, &ipco_payload)?;
    let last_property_index = next_property_index.saturating_sub(1);
    let mut ipma_entries = meta.ipma.entries.clone();
    for tile_id in tile_ids {
        ipma_entries.push(IpmaEntry {
            item_id: *tile_id,
            associations: vec![
                IpmaAssociation {
                    property_index: delta_tile_ispe_index,
                    essential: true,
                },
                IpmaAssociation {
                    property_index: primary_color_index,
                    essential: true,
                },
                IpmaAssociation {
                    property_index: delta_hvcc_index,
                    essential: true,
                },
            ],
        });
    }
    ipma_entries.push(IpmaEntry {
        item_id: delta_grid_id,
        associations: vec![
            IpmaAssociation {
                property_index: primary_color_index,
                essential: true,
            },
            IpmaAssociation {
                property_index: delta_grid_ispe_index,
                essential: false,
            },
            IpmaAssociation {
                property_index: style_pixi_index,
                essential: false,
            },
            IpmaAssociation {
                property_index: delta_auxc_index,
                essential: true,
            },
            IpmaAssociation {
                property_index: identity_irot_index,
                essential: true,
            },
        ],
    });
    ipma_entries.push(IpmaEntry {
        item_id: linear_thumbnail_id,
        associations: vec![
            IpmaAssociation {
                property_index: linear_ispe_index,
                // Native Apple Styles files mark the Linear Thumbnail's
                // geometry as essential. The decoder and auxiliary type are
                // not sufficient for ImageIO to admit the item without its
                // declared raster dimensions.
                essential: true,
            },
            IpmaAssociation {
                property_index: style_pixi_index,
                essential: false,
            },
            IpmaAssociation {
                property_index: linear_hvcc_index,
                essential: true,
            },
            IpmaAssociation {
                property_index: linear_auxc_index,
                essential: true,
            },
            IpmaAssociation {
                property_index: identity_irot_index,
                essential: true,
            },
        ],
    });

    let maximum_item_id = ipma_entries
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let ipma_version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        meta.ipma.version
    };
    let ipma_flags = if last_property_index > 0x7f {
        meta.ipma.flags | 1
    } else {
        meta.ipma.flags
    };
    let ipma = make_ipma_box(ipma_version, ipma_flags, &ipma_entries)?;

    let iprp_children = parse_boxes(source, iprp_header.payload_range())?;
    let mut payload = Vec::new();
    let mut saw_ipco = false;
    let mut saw_ipma = false;
    for header in &iprp_children {
        match header.kind {
            IPCO if !saw_ipco => {
                saw_ipco = true;
                payload.extend_from_slice(&ipco);
            }
            IPCO => return Err(invalid("source iprp contains more than one ipco")),
            IPMA if !saw_ipma => {
                saw_ipma = true;
                payload.extend_from_slice(&ipma);
            }
            IPMA => return Err(invalid("source iprp contains more than one ipma")),
            _ => payload.extend_from_slice(raw_box(source, header, "iprp child")?),
        }
    }
    if !saw_ipco || !saw_ipma {
        return Err(invalid("source iprp is missing ipco/ipma"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn build_iinf(
    source: &[u8],
    meta: &ParsedMeta,
    tile_ids: &[u32],
    delta_grid_id: u32,
    linear_thumbnail_id: u32,
    style_metadata_id: u32,
) -> Result<Vec<u8>> {
    let mut entries = Vec::new();
    for item in &meta.iinf.entries {
        entries.push(
            source
                .get(item.box_range.clone())
                .ok_or_else(|| invalid(format!("infe {} is outside source", item.item_id)))?
                .to_vec(),
        );
    }
    for tile_id in tile_ids {
        entries.push(make_infe_box(*tile_id, HVC1, 1)?);
    }
    entries.push(make_infe_box(delta_grid_id, GRID, 1)?);
    entries.push(make_infe_box(linear_thumbnail_id, HVC1, 1)?);
    entries.push(make_uri_infe_box(style_metadata_id)?);

    let version = if meta.iinf.version == 0 && entries.len() > usize::from(u16::MAX) {
        1
    } else {
        meta.iinf.version
    };
    Ok(make_iinf_box(version, &entries)?)
}

fn build_iref(
    meta: &ParsedMeta,
    tile_ids: &[u32],
    delta_grid_id: u32,
    linear_thumbnail_id: u32,
    style_metadata_id: u32,
) -> Result<Vec<u8>> {
    let mut entries = meta
        .iref
        .as_ref()
        .map_or_else(Vec::new, |iref| iref.entries.clone());
    entries.push(IrefEntry {
        kind: DIMG,
        from_item_id: delta_grid_id,
        to_item_ids: tile_ids.to_vec(),
    });
    entries.push(IrefEntry {
        kind: AUXL,
        from_item_id: delta_grid_id,
        to_item_ids: vec![meta.primary_item_id, find_tmap_id(meta)?],
    });
    entries.push(IrefEntry {
        kind: AUXL,
        from_item_id: linear_thumbnail_id,
        to_item_ids: vec![meta.primary_item_id, find_tmap_id(meta)?],
    });
    entries.push(IrefEntry {
        kind: CDSC,
        from_item_id: style_metadata_id,
        to_item_ids: vec![meta.primary_item_id, find_tmap_id(meta)?],
    });
    let maximum_item_id = entries
        .iter()
        .flat_map(|entry| {
            std::iter::once(entry.from_item_id).chain(entry.to_item_ids.iter().copied())
        })
        .max()
        .unwrap_or(0);
    let version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        meta.iref.as_ref().map_or(0, |iref| iref.version)
    };
    Ok(make_iref_box(version, &entries)?)
}

fn find_tmap_id(meta: &ParsedMeta) -> Result<u32> {
    let mut ids = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(TMAP))
        .map(|item| item.item_id);
    let Some(id) = ids.next() else {
        return Err(invalid("Photographic Styles source has no tmap item"));
    };
    if ids.next().is_some() {
        return Err(invalid(
            "Photographic Styles source has multiple tmap items",
        ));
    }
    Ok(id)
}

fn entity_group_ids(source: &[u8], children: &[BoxHeader]) -> Result<Vec<u32>> {
    let mut group_ids = Vec::new();
    for grpl in children.iter().filter(|header| header.kind == GRPL) {
        for group in parse_boxes(source, grpl.payload_range())? {
            // Entity-group boxes are FullBoxes. The group identifier follows
            // their four-byte version/flags field. Count every group type,
            // matching Apple's writer, because a group ID occupies the same
            // namespace as the item IDs used by the appended Styles graph.
            let group_id_start = group
                .data_start
                .checked_add(4)
                .ok_or_else(|| invalid("entity group ID offset overflows"))?;
            let group_id_end = group_id_start
                .checked_add(4)
                .ok_or_else(|| invalid("entity group ID range overflows"))?;
            let bytes = source
                .get(group_id_start..group_id_end)
                .ok_or_else(|| invalid("entity group ID is truncated"))?;
            group_ids.push(u32::from_be_bytes(
                bytes
                    .try_into()
                    .map_err(|_| invalid("entity group ID has invalid width"))?,
            ));
        }
    }
    Ok(group_ids)
}

fn existing_idat_payload<'a>(source: &'a [u8], meta: &ParsedMeta) -> Result<&'a [u8]> {
    match meta.idat.as_ref() {
        Some(idat) => source
            .get(idat.payload_range())
            .ok_or_else(|| invalid("source idat payload is outside source")),
        None => Ok(&[]),
    }
}

fn normalized_existing_location(entry: &IlocEntry, placeholder: bool) -> Result<IlocEntry> {
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "item {} uses unsupported data_reference_index {}",
            entry.item_id, entry.data_reference_index
        )));
    }
    if !matches!(entry.construction_method, 0 | 1) {
        return Err(invalid(format!(
            "item {} uses unsupported construction_method {}",
            entry.item_id, entry.construction_method
        )));
    }
    let mut extents = Vec::with_capacity(entry.extents.len());
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "item {} uses unsupported non-zero extent_index",
                entry.item_id
            )));
        }
        extents.push(IlocExtent {
            index: None,
            offset: if placeholder && entry.construction_method == 0 {
                0
            } else {
                entry.resolved_extent_offset(extent)?
            },
            length: extent.length,
        });
    }
    Ok(IlocEntry {
        item_id: entry.item_id,
        construction_method: entry.construction_method,
        data_reference_index: 0,
        base_offset: 0,
        extents,
    })
}

fn ensure_u32(value: u64, context: &str) -> Result<()> {
    if value > u64::from(u32::MAX) {
        return Err(invalid(format!("{context} exceeds 32-bit iloc support")));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct NewItemIds<'a> {
    tile_ids: &'a [u32],
    delta_grid_id: u32,
    linear_thumbnail_id: u32,
    style_metadata_id: u32,
}

#[derive(Debug, Clone, Copy)]
struct IdatLayout {
    delta_grid_offset: u64,
    delta_grid_length: u64,
    style_metadata_offset: u64,
    style_metadata_length: u64,
}

fn build_placeholder_locations(
    meta: &ParsedMeta,
    ids: NewItemIds<'_>,
    idat: IdatLayout,
    assembly: &PhotographicStylesAssembly<'_>,
) -> Result<Vec<IlocEntry>> {
    let mut entries = meta
        .iloc
        .entries
        .iter()
        .map(|entry| normalized_existing_location(entry, true))
        .collect::<Result<Vec<_>>>()?;
    for tile_id in ids.tile_ids {
        entries.push(IlocEntry {
            item_id: *tile_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: u64::try_from(assembly.style_delta_tile_payload.len())
                    .map_err(|_| invalid("Style Delta tile length exceeds u64"))?,
            }],
        });
    }
    entries.push(IlocEntry {
        item_id: ids.delta_grid_id,
        construction_method: 1,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: idat.delta_grid_offset,
            length: idat.delta_grid_length,
        }],
    });
    entries.push(IlocEntry {
        item_id: ids.linear_thumbnail_id,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: 0,
            length: u64::try_from(assembly.linear_thumbnail_payload.len())
                .map_err(|_| invalid("Linear Thumbnail length exceeds u64"))?,
        }],
    });
    entries.push(IlocEntry {
        item_id: ids.style_metadata_id,
        construction_method: 1,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: idat.style_metadata_offset,
            length: idat.style_metadata_length,
        }],
    });
    entries.sort_by_key(|entry| entry.item_id);
    Ok(entries)
}

fn build_final_locations(
    source: &[u8],
    meta: &ParsedMeta,
    mdat: &BoxHeader,
    new_mdat_data_start: u64,
    ids: NewItemIds<'_>,
    idat: IdatLayout,
    assembly: &PhotographicStylesAssembly<'_>,
) -> Result<Vec<IlocEntry>> {
    let old_mdat_start =
        u64::try_from(mdat.data_start).map_err(|_| invalid("source mdat offset exceeds u64"))?;
    let old_mdat_end =
        u64::try_from(mdat.data_end).map_err(|_| invalid("source mdat end exceeds u64"))?;
    let source_idat_len = u64::try_from(existing_idat_payload(source, meta)?.len())
        .map_err(|_| invalid("source idat length exceeds u64"))?;
    let source_mdat_payload_len = mdat
        .data_end
        .checked_sub(mdat.data_start)
        .ok_or_else(|| invalid("source mdat geometry underflows"))?;

    let mut entries = Vec::new();
    for entry in &meta.iloc.entries {
        let mut normalized = normalized_existing_location(entry, false)?;
        for extent in &mut normalized.extents {
            if normalized.construction_method == 0 {
                let end = extent.offset.checked_add(extent.length).ok_or_else(|| {
                    invalid(format!("item {} extent end overflows", entry.item_id))
                })?;
                if extent.offset < old_mdat_start || end > old_mdat_end {
                    return Err(invalid(format!(
                        "item {} has file-backed data outside the source mdat",
                        entry.item_id
                    )));
                }
                let relative = extent.offset - old_mdat_start;
                extent.offset = new_mdat_data_start.checked_add(relative).ok_or_else(|| {
                    invalid(format!("item {} relocated offset overflows", entry.item_id))
                })?;
            } else {
                let end = extent.offset.checked_add(extent.length).ok_or_else(|| {
                    invalid(format!("item {} idat extent end overflows", entry.item_id))
                })?;
                if end > source_idat_len {
                    return Err(invalid(format!(
                        "item {} has an idat extent outside the source idat",
                        entry.item_id
                    )));
                }
            }
            ensure_u32(extent.offset, "existing iloc offset")?;
            ensure_u32(extent.length, "existing iloc length")?;
        }
        entries.push(normalized);
    }

    let mut appended_offset = 0u64;
    let tile_length = u64::try_from(assembly.style_delta_tile_payload.len())
        .map_err(|_| invalid("Style Delta tile length exceeds u64"))?;
    for tile_id in ids.tile_ids {
        let offset = new_mdat_data_start
            .checked_add(
                u64::try_from(source_mdat_payload_len)
                    .map_err(|_| invalid("source mdat payload length exceeds u64"))?,
            )
            .and_then(|value| value.checked_add(appended_offset))
            .ok_or_else(|| invalid("Style Delta tile offset overflows"))?;
        ensure_u32(offset, "Style Delta tile offset")?;
        ensure_u32(tile_length, "Style Delta tile length")?;
        entries.push(IlocEntry {
            item_id: *tile_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length: tile_length,
            }],
        });
        appended_offset = appended_offset
            .checked_add(tile_length)
            .ok_or_else(|| invalid("Style Delta appended length overflows"))?;
    }

    let linear_offset = new_mdat_data_start
        .checked_add(
            u64::try_from(source_mdat_payload_len)
                .map_err(|_| invalid("source mdat payload length exceeds u64"))?,
        )
        .and_then(|value| value.checked_add(appended_offset))
        .ok_or_else(|| invalid("Linear Thumbnail offset overflows"))?;
    let linear_length = u64::try_from(assembly.linear_thumbnail_payload.len())
        .map_err(|_| invalid("Linear Thumbnail length exceeds u64"))?;
    ensure_u32(linear_offset, "Linear Thumbnail offset")?;
    ensure_u32(linear_length, "Linear Thumbnail length")?;
    entries.push(IlocEntry {
        item_id: ids.linear_thumbnail_id,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: linear_offset,
            length: linear_length,
        }],
    });

    for (item_id, offset, length) in [
        (
            ids.delta_grid_id,
            idat.delta_grid_offset,
            idat.delta_grid_length,
        ),
        (
            ids.style_metadata_id,
            idat.style_metadata_offset,
            idat.style_metadata_length,
        ),
    ] {
        ensure_u32(offset, "Photographic Styles idat offset")?;
        ensure_u32(length, "Photographic Styles idat length")?;
        entries.push(IlocEntry {
            item_id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length,
            }],
        });
    }
    entries.sort_by_key(|entry| entry.item_id);
    Ok(entries)
}

#[allow(clippy::too_many_arguments)]
fn build_meta(
    source: &[u8],
    meta_header: &BoxHeader,
    children: &[BoxHeader],
    iinf: &[u8],
    iloc: &[u8],
    iprp: &[u8],
    iref: &[u8],
    idat: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta full-box header offset overflows"))?;
    let full_header = source
        .get(meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iref = false;
    let mut saw_idat = false;
    // Keep the child order emitted by Apple's Styles writer. ISO-BMFF does
    // not require this ordering, but ImageIO's Styles consumer has accepted
    // the native sequence consistently: iloc/iinf/pitm precede the
    // properties and data/reference boxes. Unknown children remain in their
    // original relative order after the known sequence.
    let canonical_order = [HDLR, DINF, ILOC, IINF, PITM, IPRP, IDAT, IREF, GRPL];
    let mut ordered_children = Vec::with_capacity(children.len());
    for kind in canonical_order {
        ordered_children.extend(children.iter().filter(|header| header.kind == kind));
    }
    ordered_children.extend(
        children
            .iter()
            .filter(|header| !canonical_order.contains(&header.kind)),
    );
    for header in ordered_children {
        match header.kind {
            IINF => payload.extend_from_slice(iinf),
            ILOC => payload.extend_from_slice(iloc),
            IPRP => payload.extend_from_slice(iprp),
            IREF if !saw_iref => {
                saw_iref = true;
                payload.extend_from_slice(iref);
            }
            IREF => return Err(invalid("source meta contains more than one iref")),
            IDAT if !saw_idat => {
                saw_idat = true;
                payload.extend_from_slice(idat);
            }
            IDAT => return Err(invalid("source meta contains more than one idat")),
            _ => payload.extend_from_slice(raw_box(source, header, "meta child")?),
        }
    }
    if !saw_iref {
        payload.extend_from_slice(iref);
    }
    if !saw_idat {
        payload.extend_from_slice(idat);
    }
    Ok(make_box(META, &payload)?)
}

/// Add a Rust-produced Photographic Styles graph to an existing ISO HDR HEIF.
///
/// This operation is deliberately resource-oriented: it does not invoke a
/// solver, renderer, ImageIO, or an external product implementation. The
/// caller must supply the metadata and compressed resources, while Rust owns
/// the graph policy, item IDs, references, offset relocation, and structural
/// publication bytes.
pub fn assemble_photographic_styles_heif(
    source: &[u8],
    assembly: &PhotographicStylesAssembly<'_>,
) -> Result<Vec<u8>> {
    validate_assembly(assembly)?;
    let top = scan_top_level_boxes(source)?;
    let _ = one_top_level(&top.boxes, FTYP, "ftyp")?;
    let meta_header = one_top_level(&top.boxes, META, "meta")?;
    let mdat = one_top_level(&top.boxes, MDAT, "mdat")?;
    let meta = parse_meta_box(source, meta_header)?;
    find_style_metadata_item(source, &meta)?;
    let _ = find_style_graph_roots(&meta)?;
    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    if meta_children_start > meta_header.data_end {
        return Err(invalid("meta full-box header is truncated"));
    }
    let meta_children = parse_boxes(source, meta_children_start..meta_header.data_end)?;
    let iprp_header = child(&meta_children, IPRP, "meta")?;
    let _ = child(&meta_children, IINF, "meta")?;
    let _ = child(&meta_children, ILOC, "meta")?;

    let existing_maximum_id = meta
        .iinf
        .entries
        .iter()
        .map(|item| item.item_id)
        .chain(meta.iloc.entries.iter().map(|entry| entry.item_id))
        .chain(meta.iref.as_ref().into_iter().flat_map(|iref| {
            iref.entries.iter().flat_map(|entry| {
                std::iter::once(entry.from_item_id).chain(entry.to_item_ids.iter().copied())
            })
        }))
        .chain(entity_group_ids(source, &meta_children)?)
        .max()
        .ok_or_else(|| invalid("Photographic Styles source contains no item IDs"))?;
    let mut next_item_id = existing_maximum_id
        .checked_add(1)
        .ok_or_else(|| invalid("Photographic Styles item ID overflows"))?;
    let mut allocate = || -> Result<u32> {
        let id = next_item_id;
        next_item_id = next_item_id
            .checked_add(1)
            .ok_or_else(|| invalid("Photographic Styles item ID overflows"))?;
        Ok(id)
    };
    let mut tile_ids = Vec::with_capacity(usize::try_from(STYLE_DELTA_TILE_COUNT).unwrap_or(0));
    for _ in 0..STYLE_DELTA_TILE_COUNT {
        tile_ids.push(allocate()?);
    }
    let delta_grid_id = allocate()?;
    let linear_thumbnail_id = allocate()?;
    let style_metadata_id = allocate()?;
    let ids = NewItemIds {
        tile_ids: &tile_ids,
        delta_grid_id,
        linear_thumbnail_id,
        style_metadata_id,
    };
    let maximum_item_id = style_metadata_id;

    let iinf = build_iinf(
        source,
        &meta,
        &tile_ids,
        delta_grid_id,
        linear_thumbnail_id,
        style_metadata_id,
    )?;
    let iprp = build_iprp(
        source,
        iprp_header,
        &meta,
        &tile_ids,
        delta_grid_id,
        linear_thumbnail_id,
        assembly,
    )?;
    let iref = build_iref(
        &meta,
        &tile_ids,
        delta_grid_id,
        linear_thumbnail_id,
        style_metadata_id,
    )?;

    let mut idat_payload = existing_idat_payload(source, &meta)?.to_vec();
    let delta_grid_offset = u64::try_from(idat_payload.len())
        .map_err(|_| invalid("Style Delta idat offset exceeds u64"))?;
    let delta_grid_payload = make_grid_payload(
        assembly.style_delta_rows,
        assembly.style_delta_columns,
        assembly.style_delta_grid_width,
        assembly.style_delta_grid_height,
    )?;
    idat_payload.extend_from_slice(&delta_grid_payload);
    let style_metadata_offset = u64::try_from(idat_payload.len())
        .map_err(|_| invalid("Styles metadata idat offset exceeds u64"))?;
    idat_payload.extend_from_slice(assembly.style_property_list);
    let idat = make_box(IDAT, &idat_payload)?;
    let idat_layout = IdatLayout {
        delta_grid_offset,
        delta_grid_length: u64::try_from(delta_grid_payload.len())
            .map_err(|_| invalid("Style Delta grid length exceeds u64"))?,
        style_metadata_offset,
        style_metadata_length: u64::try_from(assembly.style_property_list.len())
            .map_err(|_| invalid("Styles metadata length exceeds u64"))?,
    };

    let iloc_version = if maximum_item_id > u32::from(u16::MAX) {
        2
    } else {
        meta.iloc.version.max(1)
    };
    let placeholders = build_placeholder_locations(&meta, ids, idat_layout, assembly)?;
    let placeholder_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &placeholders)?;
    let preliminary_meta = build_meta(
        source,
        meta_header,
        &meta_children,
        &iinf,
        &placeholder_iloc,
        &iprp,
        &iref,
        &idat,
    )?;

    let new_mdat_box_start = top
        .boxes
        .iter()
        .take_while(|header| header.box_start != mdat.box_start)
        .try_fold(0usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("Photographic Styles mdat offset overflows"))
        })?;
    let new_mdat_data_start = u64::try_from(
        new_mdat_box_start
            .checked_add(8)
            .ok_or_else(|| invalid("Photographic Styles mdat data offset overflows"))?,
    )
    .map_err(|_| invalid("Photographic Styles mdat data offset exceeds u64"))?;
    ensure_u32(new_mdat_data_start, "Photographic Styles mdat data offset")?;

    let final_locations = build_final_locations(
        source,
        &meta,
        mdat,
        new_mdat_data_start,
        ids,
        idat_layout,
        assembly,
    )?;
    let final_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &final_locations)?;
    let final_meta = build_meta(
        source,
        meta_header,
        &meta_children,
        &iinf,
        &final_iloc,
        &iprp,
        &iref,
        &idat,
    )?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "Photographic Styles iloc rewrite changed meta size unexpectedly",
        ));
    }

    let source_mdat_payload = source
        .get(mdat.payload_range())
        .ok_or_else(|| invalid("source mdat payload is outside source"))?;
    let tile_bytes = assembly
        .style_delta_tile_payload
        .len()
        .checked_mul(usize::try_from(STYLE_DELTA_TILE_COUNT).unwrap_or(0))
        .ok_or_else(|| invalid("Style Delta mdat payload length overflows"))?;
    let final_payload_len = source_mdat_payload
        .len()
        .checked_add(tile_bytes)
        .and_then(|length| length.checked_add(assembly.linear_thumbnail_payload.len()))
        .ok_or_else(|| invalid("Photographic Styles mdat payload length overflows"))?;
    if final_payload_len > (u32::MAX as usize).saturating_sub(8) {
        return Err(invalid(
            "Photographic Styles writer requires a 32-bit mdat size",
        ));
    }
    let mut final_mdat_payload = Vec::with_capacity(final_payload_len);
    final_mdat_payload.extend_from_slice(source_mdat_payload);
    for _ in 0..STYLE_DELTA_TILE_COUNT {
        final_mdat_payload.extend_from_slice(assembly.style_delta_tile_payload);
    }
    final_mdat_payload.extend_from_slice(assembly.linear_thumbnail_payload);
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;

    let trailing = source
        .get(top.trailing_range.clone())
        .ok_or_else(|| invalid("source trailing bytes are outside source"))?;
    let output_capacity = top.boxes.iter().try_fold(trailing.len(), |total, header| {
        let replacement_len = if header.kind == META {
            final_meta.len()
        } else if header.kind == MDAT {
            final_mdat.len()
        } else {
            header.size
        };
        total
            .checked_add(replacement_len)
            .ok_or_else(|| invalid("Photographic Styles output size overflows"))
    })?;
    let mut output = Vec::with_capacity(output_capacity);
    for header in &top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&final_mdat);
        } else {
            output.extend_from_slice(raw_box(source, header, "top-level")?);
        }
    }
    output.extend_from_slice(trailing);

    let reparsed_top = scan_top_level_boxes(&output)?;
    let reparsed_meta_header = one_top_level(&reparsed_top.boxes, META, "output meta")?;
    let reparsed_meta = parse_meta_box(&output, reparsed_meta_header)?;
    if !reparsed_meta
        .iinf
        .entries
        .iter()
        .any(|item| item.item_id == style_metadata_id && item.item_type == Some(URI))
    {
        return Err(invalid(
            "Photographic Styles output lost its style metadata item",
        ));
    }
    let style_reference = reparsed_meta
        .iref
        .as_ref()
        .map(|iref| {
            iref.entries.iter().any(|entry| {
                entry.kind == CDSC
                    && entry.from_item_id == style_metadata_id
                    && entry.to_item_ids
                        == vec![
                            reparsed_meta.primary_item_id,
                            find_tmap_id(&reparsed_meta).unwrap_or(0),
                        ]
            })
        })
        .unwrap_or(false);
    if !style_reference {
        return Err(invalid(
            "Photographic Styles output lost the style metadata descriptor reference",
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{make_ipma_box, make_iref_box, make_pitm_box};

    const PRIMARY_ID: u32 = 1;
    const GAIN_ID: u32 = 2;
    const TMAP_ID: u32 = 3;
    const TILE_ID: u32 = 4;

    fn colr_box() -> Vec<u8> {
        let mut payload = Vec::from(*b"nclx");
        payload.extend_from_slice(&9u16.to_be_bytes());
        payload.extend_from_slice(&16u16.to_be_bytes());
        payload.extend_from_slice(&9u16.to_be_bytes());
        payload.push(0x80);
        make_box(COLR, &payload).unwrap()
    }

    fn pixi_box() -> Vec<u8> {
        make_full_box(PIXI, 0, 0, &[3, 8, 8, 8]).unwrap()
    }

    fn fixture() -> Vec<u8> {
        let primary = b"primary";
        let gain = b"gain";
        let tmap = b"tmap";
        let tile = b"tile";
        let mut idat_payload = Vec::new();
        let primary_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(primary);
        let gain_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(gain);
        let tmap_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(tmap);
        let tile_offset = idat_payload.len() as u64;
        idat_payload.extend_from_slice(tile);

        let mut ipco_payload = Vec::new();
        for property in [
            make_ispe_box(4, 4).unwrap(),
            colr_box(),
            pixi_box(),
            make_ispe_box(4, 4).unwrap(),
            make_box(HVCC, &[1; 19]).unwrap(),
        ] {
            ipco_payload.extend_from_slice(&property);
        }
        let ipco = make_box(IPCO, &ipco_payload).unwrap();
        let ipma = make_ipma_box(
            0,
            0,
            &[
                IpmaEntry {
                    item_id: PRIMARY_ID,
                    associations: vec![IpmaAssociation {
                        property_index: 2,
                        essential: true,
                    }],
                },
                IpmaEntry {
                    item_id: GAIN_ID,
                    associations: vec![IpmaAssociation {
                        property_index: 1,
                        essential: true,
                    }],
                },
                IpmaEntry {
                    item_id: TMAP_ID,
                    associations: vec![IpmaAssociation {
                        property_index: 4,
                        essential: true,
                    }],
                },
                IpmaEntry {
                    item_id: TILE_ID,
                    associations: vec![IpmaAssociation {
                        property_index: 5,
                        essential: true,
                    }],
                },
            ],
        )
        .unwrap();
        let iprp = make_box(IPRP, &[ipco, ipma].concat()).unwrap();
        let iinf = make_iinf_box(
            0,
            &[
                make_infe_box(PRIMARY_ID, HVC1, 0).unwrap(),
                make_infe_box(GAIN_ID, GRID, 1).unwrap(),
                make_infe_box(TMAP_ID, TMAP, 0).unwrap(),
                make_infe_box(TILE_ID, HVC1, 1).unwrap(),
            ],
        )
        .unwrap();
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
                        length: primary.len() as u64,
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
                        length: gain.len() as u64,
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
                        length: tmap.len() as u64,
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
                        length: tile.len() as u64,
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
                    to_item_ids: vec![PRIMARY_ID, GAIN_ID],
                },
                IrefEntry {
                    kind: DIMG,
                    from_item_id: GAIN_ID,
                    to_item_ids: vec![TILE_ID],
                },
            ],
        )
        .unwrap();
        let pitm = make_pitm_box(0, PRIMARY_ID).unwrap();
        let idat = make_box(IDAT, &idat_payload).unwrap();
        let mut altr_payload = vec![0, 0, 0, 0];
        altr_payload.extend_from_slice(&5_u32.to_be_bytes());
        altr_payload.extend_from_slice(&2_u32.to_be_bytes());
        altr_payload.extend_from_slice(&TMAP_ID.to_be_bytes());
        altr_payload.extend_from_slice(&PRIMARY_ID.to_be_bytes());
        let grpl = make_box(
            GRPL,
            &make_box(FourCC::new(*b"altr"), &altr_payload).unwrap(),
        )
        .unwrap();
        let mut meta_payload = vec![0, 0, 0, 0];
        for part in [pitm, iinf, iloc, iprp, iref, idat, grpl] {
            meta_payload.extend_from_slice(&part);
        }
        [
            make_box(FTYP, b"mif1\0\0\0\0").unwrap(),
            make_box(META, &meta_payload).unwrap(),
            make_box(MDAT, b"source-mdat").unwrap(),
        ]
        .concat()
    }

    fn assembly<'a>(plist: &'a [u8]) -> PhotographicStylesAssembly<'a> {
        PhotographicStylesAssembly {
            style_property_list: plist,
            style_delta_hvcc: &[1; 19],
            style_delta_tile_payload: b"delta-tile",
            style_delta_tile_width: 2,
            style_delta_tile_height: 2,
            style_delta_grid_width: 10,
            style_delta_grid_height: 6,
            style_delta_rows: 5,
            style_delta_columns: 6,
            linear_thumbnail_hvcc: &[2; 19],
            linear_thumbnail_payload: b"linear-thumbnail",
            linear_thumbnail_width: 4,
            linear_thumbnail_height: 4,
        }
    }

    #[test]
    fn derives_primary_color_from_grid_components_when_grid_has_geometry_only() {
        let source = fixture();
        let top = scan_top_level_boxes(&source).unwrap();
        let meta_header = one_top_level(&top.boxes, META, "fixture meta").unwrap();
        let mut meta = parse_meta_box(&source, meta_header).unwrap();
        meta.ipma
            .entries
            .iter_mut()
            .find(|entry| entry.item_id == PRIMARY_ID)
            .unwrap()
            .associations
            .retain(|association| association.property_index != 2);
        meta.ipma
            .entries
            .iter_mut()
            .find(|entry| entry.item_id == TILE_ID)
            .unwrap()
            .associations
            .push(IpmaAssociation {
                property_index: 2,
                essential: true,
            });
        meta.iref.as_mut().unwrap().entries.push(IrefEntry {
            kind: DIMG,
            from_item_id: PRIMARY_ID,
            to_item_ids: vec![TILE_ID],
        });
        assert_eq!(style_delta_color_property_index(&meta).unwrap(), 2);
    }

    #[test]
    fn appends_rust_resources_and_preserves_source_mdat_prefix() {
        let output =
            assemble_photographic_styles_heif(&fixture(), &assembly(b"bplist00test")).unwrap();
        let top = scan_top_level_boxes(&output).unwrap();
        let mdat = one_top_level(&top.boxes, MDAT, "output mdat").unwrap();
        assert!(output[mdat.payload_range()].starts_with(b"source-mdat"));
        let meta_header = one_top_level(&top.boxes, META, "output meta").unwrap();
        let meta = parse_meta_box(&output, meta_header).unwrap();
        assert_eq!(meta.iinf.entries.len(), 4 + 33);
        assert!(!meta.iinf.entries.iter().any(|item| item.item_id == 5));
        assert_eq!(
            meta.iinf
                .entries
                .iter()
                .map(|item| item.item_id)
                .filter(|item_id| *item_id > TILE_ID)
                .min(),
            Some(6)
        );
        assert!(meta
            .iinf
            .entries
            .iter()
            .any(|item| item.item_type == Some(URI)));
        let style_id = meta
            .iinf
            .entries
            .iter()
            .find(|item| item.item_type == Some(URI))
            .unwrap()
            .item_id;
        assert!(meta
            .iref
            .unwrap()
            .entries
            .iter()
            .any(|entry| { entry.kind == CDSC && entry.from_item_id == style_id }));
    }

    #[test]
    fn rejects_non_binary_style_metadata_before_parsing_source() {
        let assembly = assembly(b"not-a-plist");
        assert!(assemble_photographic_styles_heif(&[], &assembly).is_err());
    }

    #[test]
    fn rejects_a_second_style_metadata_item() {
        let source = fixture();
        let assembly = assembly(b"bplist00test");
        let output = assemble_photographic_styles_heif(&source, &assembly).unwrap();
        assert!(assemble_photographic_styles_heif(&output, &assembly).is_err());
    }
}

use std::collections::{BTreeMap, BTreeSet};

use xdremux_format::isobmff::{
    make_box, make_iinf_box, make_iloc_box, make_ipma_box, make_iref_box, parse_boxes,
    parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry, IlocExtent, IpmaAssociation,
    IpmaEntry, IrefEntry, ParsedMeta, PropertyInfo, IDAT, IINF, ILOC, INFE, IPCO, IPMA, IPRP, IREF,
    MDAT, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const GRID: FourCC = FourCC::new(*b"grid");
const MIME: FourCC = FourCC::new(*b"mime");
const EXIF: FourCC = FourCC::new(*b"Exif");
const TMAP: FourCC = FourCC::new(*b"tmap");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const DIMG: FourCC = FourCC::new(*b"dimg");
const COLR: FourCC = FourCC::new(*b"colr");
const IROT: FourCC = FourCC::new(*b"irot");

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn raw_box<'a>(data: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    data.get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside input")))
}

fn raw_property<'a>(data: &'a [u8], property: &PropertyInfo) -> Result<&'a [u8]> {
    data.get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside input", property.index)))
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

#[derive(Debug, Clone)]
struct Graph {
    top: xdremux_format::isobmff::TopLevelScan,
    meta_header: BoxHeader,
    meta_children: Vec<BoxHeader>,
    meta: ParsedMeta,
    mdat: BoxHeader,
}

fn parse_graph(data: &[u8], owner: &str) -> Result<Graph> {
    let top = scan_top_level_boxes(data)?;
    let meta_header = one_top_level(&top.boxes, META, &format!("{owner} meta"))?.clone();
    let mdat = one_top_level(&top.boxes, MDAT, &format!("{owner} mdat"))?.clone();
    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid(format!("{owner} meta header offset overflows")))?;
    if meta_children_start > meta_header.data_end {
        return Err(invalid(format!(
            "{owner} meta full-box header is truncated"
        )));
    }
    let meta_children = parse_boxes(data, meta_children_start..meta_header.data_end)?;
    let meta = parse_meta_box(data, &meta_header)?;
    for kind in [IINF, ILOC, IPRP] {
        let _ = child(&meta_children, kind, owner)?;
    }
    Ok(Graph {
        top,
        meta_header,
        meta_children,
        meta,
        mdat,
    })
}

fn find_single_item(meta: &ParsedMeta, item_type: FourCC, context: &str) -> Result<u32> {
    let ids = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(item_type))
        .map(|item| item.item_id)
        .collect::<Vec<_>>();
    match ids.as_slice() {
        [only] => Ok(*only),
        [] => Err(invalid(format!("{context} has no {item_type} item"))),
        _ => Err(invalid(format!("{context} has multiple {item_type} items"))),
    }
}

fn find_tmap(meta: &ParsedMeta, context: &str) -> Result<u32> {
    find_single_item(meta, TMAP, context)
}

fn item_info<'a>(
    meta: &'a ParsedMeta,
    item_id: u32,
    context: &str,
) -> Result<&'a xdremux_format::isobmff::ItemInfo> {
    meta.iinf
        .entries
        .iter()
        .find(|item| item.item_id == item_id)
        .ok_or_else(|| invalid(format!("{context} item {item_id} is missing")))
}

fn item_location<'a>(meta: &'a ParsedMeta, item_id: u32, context: &str) -> Result<&'a IlocEntry> {
    meta.iloc
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .ok_or_else(|| invalid(format!("{context} item {item_id} has no iloc entry")))
}

fn item_payload(data: &[u8], graph: &Graph, item_id: u32, context: &str) -> Result<Vec<u8>> {
    let entry = item_location(&graph.meta, item_id, context)?;
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "{context} item {item_id} uses data_reference_index {}",
            entry.data_reference_index
        )));
    }
    let mut payload = Vec::new();
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "{context} item {item_id} uses a non-zero extent index"
            )));
        }
        let offset = entry.resolved_extent_offset(extent)?;
        let end = offset
            .checked_add(extent.length)
            .ok_or_else(|| invalid(format!("{context} item {item_id} extent overflows")))?;
        let range = match entry.construction_method {
            0 => {
                let start = u64::try_from(graph.mdat.data_start)
                    .map_err(|_| invalid(format!("{context} mdat offset exceeds u64")))?;
                let limit = u64::try_from(graph.mdat.data_end)
                    .map_err(|_| invalid(format!("{context} mdat end exceeds u64")))?;
                if offset < start || end > limit {
                    return Err(invalid(format!(
                        "{context} item {item_id} extent is outside mdat"
                    )));
                }
                let start = usize::try_from(offset).map_err(|_| {
                    invalid(format!("{context} item {item_id} offset exceeds usize"))
                })?;
                let end = usize::try_from(end)
                    .map_err(|_| invalid(format!("{context} item {item_id} end exceeds usize")))?;
                start..end
            }
            1 => {
                let idat = graph
                    .meta
                    .idat
                    .as_ref()
                    .ok_or_else(|| invalid(format!("{context} item {item_id} has no idat")))?;
                let idat_start = u64::try_from(idat.data_start)
                    .map_err(|_| invalid(format!("{context} idat offset exceeds u64")))?;
                let idat_length = u64::try_from(idat.data_end - idat.data_start)
                    .map_err(|_| invalid(format!("{context} idat length exceeds u64")))?;
                if end > idat_length {
                    return Err(invalid(format!(
                        "{context} item {item_id} idat extent is outside idat"
                    )));
                }
                let start = usize::try_from(idat_start.checked_add(offset).ok_or_else(|| {
                    invalid(format!("{context} item {item_id} idat offset overflows"))
                })?)
                .map_err(|_| invalid(format!("{context} item {item_id} offset exceeds usize")))?;
                let end = usize::try_from(idat_start.checked_add(end).ok_or_else(|| {
                    invalid(format!("{context} item {item_id} idat end overflows"))
                })?)
                .map_err(|_| invalid(format!("{context} item {item_id} end exceeds usize")))?;
                start..end
            }
            method => {
                return Err(invalid(format!(
                    "{context} item {item_id} uses unsupported construction_method {method}"
                )))
            }
        };
        payload.extend_from_slice(
            data.get(range).ok_or_else(|| {
                invalid(format!("{context} item {item_id} extent is outside input"))
            })?,
        );
    }
    if payload.is_empty() {
        return Err(invalid(format!(
            "{context} item {item_id} has an empty payload"
        )));
    }
    Ok(payload)
}

fn normalized_source_location(
    data: &[u8],
    graph: &Graph,
    entry: &IlocEntry,
    placeholder: bool,
) -> Result<IlocEntry> {
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "source item {} uses data_reference_index {}",
            entry.item_id, entry.data_reference_index
        )));
    }
    if !matches!(entry.construction_method, 0 | 1) {
        return Err(invalid(format!(
            "source item {} uses unsupported construction_method {}",
            entry.item_id, entry.construction_method
        )));
    }
    let mut extents = Vec::with_capacity(entry.extents.len());
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "source item {} uses a non-zero extent index",
                entry.item_id
            )));
        }
        let offset = if placeholder && entry.construction_method == 0 {
            0
        } else {
            let resolved = entry.resolved_extent_offset(extent)?;
            if entry.construction_method == 0 {
                let mdat_start = u64::try_from(graph.mdat.data_start)
                    .map_err(|_| invalid("source mdat offset exceeds u64"))?;
                let mdat_end = u64::try_from(graph.mdat.data_end)
                    .map_err(|_| invalid("source mdat end exceeds u64"))?;
                let end = resolved
                    .checked_add(extent.length)
                    .ok_or_else(|| invalid("source mdat extent overflows"))?;
                if resolved < mdat_start || end > mdat_end {
                    return Err(invalid(format!(
                        "source item {} has file data outside mdat",
                        entry.item_id
                    )));
                }
            } else {
                let idat = graph
                    .meta
                    .idat
                    .as_ref()
                    .ok_or_else(|| invalid("source construction-method-1 item has no idat"))?;
                let idat_len = u64::try_from(idat.data_end - idat.data_start)
                    .map_err(|_| invalid("source idat length exceeds u64"))?;
                if resolved
                    .checked_add(extent.length)
                    .is_none_or(|end| end > idat_len)
                {
                    return Err(invalid(format!(
                        "source item {} has idat data outside idat",
                        entry.item_id
                    )));
                }
            }
            resolved
        };
        extents.push(IlocExtent {
            index: None,
            offset,
            length: extent.length,
        });
    }
    let _ = data;
    Ok(IlocEntry {
        item_id: entry.item_id,
        construction_method: entry.construction_method,
        data_reference_index: 0,
        base_offset: 0,
        extents,
    })
}

fn source_maximum_item_id(meta: &ParsedMeta) -> Result<u32> {
    meta.iinf
        .entries
        .iter()
        .map(|item| item.item_id)
        .chain(meta.iloc.entries.iter().map(|entry| entry.item_id))
        .chain(meta.iref.as_ref().into_iter().flat_map(|iref| {
            iref.entries.iter().flat_map(|reference| {
                std::iter::once(reference.from_item_id).chain(reference.to_item_ids.iter().copied())
            })
        }))
        .max()
        .ok_or_else(|| invalid("source semantic graph has no item IDs"))
}

fn semantic_items(scaffold: &Graph) -> Result<(u32, Vec<u32>, Vec<u32>)> {
    let scaffold_tmap = find_tmap(&scaffold.meta, "semantic scaffold")?;
    let scaffold_primary = scaffold.meta.primary_item_id;
    let mut images = Vec::new();
    let mut seen_images = BTreeSet::new();
    if let Some(iref) = scaffold.meta.iref.as_ref() {
        for reference in &iref.entries {
            if reference.kind != AUXL
                || !reference.to_item_ids.contains(&scaffold_primary)
                || !reference.to_item_ids.contains(&scaffold_tmap)
                || item_info(&scaffold.meta, reference.from_item_id, "semantic scaffold")?.item_type
                    != Some(HVC1)
            {
                continue;
            }
            if seen_images.insert(reference.from_item_id) {
                images.push(reference.from_item_id);
            }
        }
    }
    if images.is_empty() {
        return Err(invalid(
            "semantic scaffold has no auxiliary semantic image items",
        ));
    }
    let image_set = images.iter().copied().collect::<BTreeSet<_>>();
    let mut metadata = Vec::new();
    let mut seen_metadata = BTreeSet::new();
    if let Some(iref) = scaffold.meta.iref.as_ref() {
        for reference in &iref.entries {
            if reference.kind != CDSC
                || reference.to_item_ids.len() != 1
                || !image_set.contains(&reference.to_item_ids[0])
                || item_info(&scaffold.meta, reference.from_item_id, "semantic scaffold")?.item_type
                    != Some(MIME)
            {
                continue;
            }
            if seen_metadata.insert(reference.from_item_id) {
                metadata.push(reference.from_item_id);
            }
        }
    }
    if metadata.len() != images.len() {
        return Err(invalid(format!(
            "semantic scaffold has {} semantic images but {} metadata descriptors",
            images.len(),
            metadata.len()
        )));
    }
    Ok((scaffold_tmap, images, metadata))
}

fn associated_properties(meta: &ParsedMeta, item_id: u32) -> Result<Vec<IpmaAssociation>> {
    meta.ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .map(|entry| entry.associations.clone())
        .ok_or_else(|| {
            invalid(format!(
                "semantic scaffold item {item_id} has no ipma entry"
            ))
        })
}

fn property_index_of_kind(meta: &ParsedMeta, item_id: u32, kind: FourCC) -> Option<u16> {
    meta.ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .and_then(|entry| {
            entry.associations.iter().find_map(|association| {
                meta.properties
                    .iter()
                    .find(|property| property.index == u32::from(association.property_index))
                    .filter(|property| property.kind == kind)
                    .map(|_| association.property_index)
            })
        })
}

fn source_primary_color_property_index(meta: &ParsedMeta) -> Result<Option<u16>> {
    if let Some(index) = property_index_of_kind(meta, meta.primary_item_id, COLR) {
        return Ok(Some(index));
    }

    // A Rust-authored base may associate the primary color profile with the
    // primary grid's hvc1 children rather than with the grid item itself. A
    // semantic ImageIO write expects the primary association to remain
    // explicit, so recover it only from the primary grid's own dimg edge and
    // require all participating components to agree on one property.
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
                if is_primary_component {
                    if let Some(index) = property_index_of_kind(meta, *item_id, COLR) {
                        candidates.insert(index);
                    }
                }
            }
        }
    }
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.into_iter().next()),
        _ => Err(invalid(
            "semantic merge found ambiguous primary component color properties",
        )),
    }
}

fn first_property_index_of_kind(meta: &ParsedMeta, kind: FourCC) -> Option<u16> {
    meta.properties
        .iter()
        .find(|property| property.kind == kind)
        .and_then(|property| u16::try_from(property.index).ok())
}

fn make_iprp(
    source_data: &[u8],
    scaffold_data: &[u8],
    source: &Graph,
    scaffold: &Graph,
    imported_images: &[(u32, u32)],
) -> Result<Vec<u8>> {
    let source_iprp = child(&source.meta_children, IPRP, "source meta")?;
    let scaffold_property_by_index = scaffold
        .meta
        .properties
        .iter()
        .map(|property| (property.index, property))
        .collect::<BTreeMap<_, _>>();
    let mut properties = source.meta.properties.iter().collect::<Vec<_>>();
    properties.sort_by_key(|property| property.index);
    for (position, property) in properties.iter().enumerate() {
        let expected = u32::try_from(position + 1)
            .map_err(|_| invalid("source ipco property index exceeds u32"))?;
        if property.index != expected {
            return Err(invalid("source ipco property indices are not contiguous"));
        }
    }

    let mut ipco_payload = Vec::new();
    for property in &properties {
        ipco_payload.extend_from_slice(raw_property(source_data, property)?);
    }
    let mut next_property_index = u16::try_from(properties.len() + 1)
        .map_err(|_| invalid("source ipco property count exceeds u16"))?;
    let mut property_map = BTreeMap::new();
    let mut imported_associations = Vec::with_capacity(imported_images.len());
    for (source_id, scaffold_id) in imported_images {
        let associations = associated_properties(&scaffold.meta, *scaffold_id)?;
        let mut mapped = Vec::with_capacity(associations.len());
        for association in associations {
            let new_index = if let Some(index) = property_map.get(&association.property_index) {
                *index
            } else {
                let property = scaffold_property_by_index
                    .get(&u32::from(association.property_index))
                    .ok_or_else(|| {
                        invalid(format!(
                            "semantic scaffold property {} is missing",
                            association.property_index
                        ))
                    })?;
                let index = next_property_index;
                ipco_payload.extend_from_slice(raw_property(scaffold_data, property)?);
                next_property_index = next_property_index
                    .checked_add(1)
                    .ok_or_else(|| invalid("semantic property index overflows"))?;
                property_map.insert(association.property_index, index);
                index
            };
            mapped.push(IpmaAssociation {
                property_index: new_index,
                essential: association.essential,
            });
        }
        imported_associations.push(IpmaEntry {
            item_id: *source_id,
            associations: mapped,
        });
    }
    let last_property_index = next_property_index.saturating_sub(1);
    let mut ipma_entries = source.meta.ipma.entries.clone();
    if let Some(primary_entry) = ipma_entries
        .iter_mut()
        .find(|entry| entry.item_id == source.meta.primary_item_id)
    {
        if let Some(irot_index) = first_property_index_of_kind(&source.meta, IROT) {
            if !primary_entry
                .associations
                .iter()
                .any(|association| association.property_index == irot_index)
            {
                primary_entry.associations.push(IpmaAssociation {
                    property_index: irot_index,
                    essential: true,
                });
            }
        }
        if let Some(color_index) = source_primary_color_property_index(&source.meta)? {
            if !primary_entry
                .associations
                .iter()
                .any(|association| association.property_index == color_index)
            {
                primary_entry.associations.push(IpmaAssociation {
                    property_index: color_index,
                    essential: true,
                });
            }
        }
    }
    ipma_entries.extend(imported_associations);
    let maximum_item_id = ipma_entries
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let ipma_version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        source.meta.ipma.version
    };
    let ipma_flags = if last_property_index > 0x7f {
        source.meta.ipma.flags | 1
    } else {
        source.meta.ipma.flags
    };
    let ipco = make_box(IPCO, &ipco_payload)?;
    let ipma = make_ipma_box(ipma_version, ipma_flags, &ipma_entries)?;
    let iprp_children = parse_boxes(source_data, source_iprp.payload_range())?;
    let mut payload = Vec::new();
    let mut saw_ipco = false;
    let mut saw_ipma = false;
    for header in &iprp_children {
        match header.kind {
            IPCO if !saw_ipco => {
                saw_ipco = true;
                payload.extend_from_slice(&ipco);
            }
            IPCO => return Err(invalid("source iprp contains multiple ipco boxes")),
            IPMA if !saw_ipma => {
                saw_ipma = true;
                payload.extend_from_slice(&ipma);
            }
            IPMA => return Err(invalid("source iprp contains multiple ipma boxes")),
            _ => payload.extend_from_slice(raw_box(source_data, header, "source iprp child")?),
        }
    }
    if !saw_ipco || !saw_ipma {
        return Err(invalid("source iprp is missing ipco or ipma"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn remap_infe_item_id(
    data: &[u8],
    item: &xdremux_format::isobmff::ItemInfo,
    new_id: u32,
) -> Result<Vec<u8>> {
    let raw = data.get(item.box_range.clone()).ok_or_else(|| {
        invalid(format!(
            "semantic scaffold infe {} is outside input",
            item.item_id
        ))
    })?;
    if raw.get(4..8) != Some(INFE.as_bytes()) || raw.len() < 14 {
        return Err(invalid(format!(
            "semantic scaffold item {} has a malformed infe box",
            item.item_id
        )));
    }
    let version = raw[8];
    let mut output = raw.to_vec();
    match version {
        2 => {
            let id = u16::try_from(new_id).map_err(|_| {
                invalid(format!(
                    "semantic scaffold item {} cannot be remapped into infe version 2",
                    item.item_id
                ))
            })?;
            output[12..14].copy_from_slice(&id.to_be_bytes());
        }
        3 => {
            if output.len() < 16 {
                return Err(invalid(format!(
                    "semantic scaffold item {} has a truncated infe version 3 box",
                    item.item_id
                )));
            }
            output[12..16].copy_from_slice(&new_id.to_be_bytes());
        }
        other => {
            return Err(invalid(format!(
                "semantic scaffold item {} uses unsupported infe version {other}",
                item.item_id
            )))
        }
    }
    Ok(output)
}

fn make_iinf(
    source_data: &[u8],
    scaffold_data: &[u8],
    source: &Graph,
    scaffold: &Graph,
    records: &[(u32, u32)],
) -> Result<Vec<u8>> {
    let mut entries = Vec::new();
    for item in &source.meta.iinf.entries {
        entries.push(
            source_data
                .get(item.box_range.clone())
                .ok_or_else(|| invalid(format!("source infe {} is outside input", item.item_id)))?
                .to_vec(),
        );
    }
    for (new_id, old_id) in records {
        let info = item_info(&scaffold.meta, *old_id, "semantic scaffold")?;
        entries.push(remap_infe_item_id(scaffold_data, info, *new_id)?);
    }
    let version = if source.meta.iinf.version == 0 && entries.len() > usize::from(u16::MAX) {
        1
    } else {
        source.meta.iinf.version
    };
    Ok(make_iinf_box(version, &entries)?)
}

fn map_semantic_reference(
    reference: &IrefEntry,
    scaffold_primary: u32,
    scaffold_tmap: u32,
    source_primary: u32,
    source_tmap: u32,
    id_map: &BTreeMap<u32, u32>,
) -> Result<IrefEntry> {
    let from_item_id = *id_map.get(&reference.from_item_id).ok_or_else(|| {
        invalid(format!(
            "semantic reference source item {} is not imported",
            reference.from_item_id
        ))
    })?;
    let mut to_item_ids = Vec::with_capacity(reference.to_item_ids.len());
    for target in &reference.to_item_ids {
        let mapped = if *target == scaffold_primary {
            source_primary
        } else if *target == scaffold_tmap {
            source_tmap
        } else if let Some(mapped) = id_map.get(target) {
            *mapped
        } else {
            return Err(invalid(format!(
                "semantic reference target item {target} is not remappable"
            )));
        };
        to_item_ids.push(mapped);
    }
    Ok(IrefEntry {
        kind: reference.kind,
        from_item_id,
        to_item_ids,
    })
}

fn make_meta(
    source_data: &[u8],
    source: &Graph,
    iinf: &[u8],
    iloc: &[u8],
    iprp: &[u8],
    iref: &[u8],
    idat: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = source
        .meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("source meta full-box header offset overflows"))?;
    let full_header = source_data
        .get(source.meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("source meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iref = false;
    let mut saw_idat = false;
    for header in &source.meta_children {
        match header.kind {
            IINF => payload.extend_from_slice(iinf),
            ILOC => payload.extend_from_slice(iloc),
            IPRP => payload.extend_from_slice(iprp),
            IREF if !saw_iref => {
                saw_iref = true;
                payload.extend_from_slice(iref);
            }
            IREF => return Err(invalid("source meta contains multiple iref boxes")),
            IDAT if !saw_idat => {
                saw_idat = true;
                payload.extend_from_slice(idat);
            }
            IDAT => return Err(invalid("source meta contains multiple idat boxes")),
            _ => payload.extend_from_slice(raw_box(source_data, header, "source meta child")?),
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

fn normalized_source_final_location(
    source_data: &[u8],
    source: &Graph,
    entry: &IlocEntry,
    new_mdat_data_start: u64,
) -> Result<IlocEntry> {
    let mut normalized = normalized_source_location(source_data, source, entry, false)?;
    if normalized.construction_method == 0 {
        let old_mdat_start = u64::try_from(source.mdat.data_start)
            .map_err(|_| invalid("source mdat offset exceeds u64"))?;
        for extent in &mut normalized.extents {
            let relative = extent
                .offset
                .checked_sub(old_mdat_start)
                .ok_or_else(|| invalid("source file extent precedes mdat"))?;
            extent.offset = new_mdat_data_start
                .checked_add(relative)
                .ok_or_else(|| invalid("relocated semantic extent offset overflows"))?;
        }
    }
    Ok(normalized)
}

/// Merge semantic image resources produced by the ImageIO write primitive
/// into the Rust-owned HDR graph while retaining the source base/Gain Map
/// payloads byte-for-byte.
///
/// ImageIO is intentionally used only to encode and describe the semantic
/// resources. Re-running its destination writer over a Rust-authored ISO Gain
/// Map can hide the Gain Map from ImageIO when an auxiliary matte is added.
/// This graph merge is therefore part of the Rust container ownership layer:
/// it imports only the scaffold's semantic items, properties, and references,
/// and keeps the source primary/tmap items and mdat prefix authoritative.
pub fn merge_apple_semantic_auxiliary_heif(
    source_data: &[u8],
    scaffold_data: &[u8],
    expected_semantic_roles: usize,
) -> Result<Vec<u8>> {
    if expected_semantic_roles == 0 {
        return Err(invalid("semantic merge requires at least one role"));
    }
    let source = parse_graph(source_data, "source HDR")?;
    let scaffold = parse_graph(scaffold_data, "semantic scaffold")?;
    let source_tmap = find_tmap(&source.meta, "source HDR")?;
    let (scaffold_tmap, semantic_images, semantic_metadata) = semantic_items(&scaffold)?;
    if semantic_images.len() != expected_semantic_roles {
        return Err(invalid(format!(
            "semantic scaffold contains {} roles; expected {expected_semantic_roles}",
            semantic_images.len()
        )));
    }

    let source_exif = find_single_item(&source.meta, EXIF, "source HDR")?;
    let scaffold_exif = find_single_item(&scaffold.meta, EXIF, "semantic scaffold")?;
    let scaffold_exif_payload = item_payload(
        scaffold_data,
        &scaffold,
        scaffold_exif,
        "semantic scaffold Exif",
    )?;

    let mut next_item_id = source_maximum_item_id(&source.meta)?
        .checked_add(1)
        .ok_or_else(|| invalid("semantic merge item ID overflows"))?;
    let mut id_map = BTreeMap::new();
    for old_id in semantic_images
        .iter()
        .chain(semantic_metadata.iter())
        .copied()
    {
        if id_map.contains_key(&old_id) {
            continue;
        }
        id_map.insert(old_id, next_item_id);
        next_item_id = next_item_id
            .checked_add(1)
            .ok_or_else(|| invalid("semantic merge item ID overflows"))?;
    }

    let mut records = Vec::with_capacity(semantic_images.len() + semantic_metadata.len());
    for old_id in semantic_images
        .iter()
        .chain(semantic_metadata.iter())
        .copied()
    {
        let new_id = *id_map
            .get(&old_id)
            .ok_or_else(|| invalid(format!("semantic item {old_id} has no allocated ID")))?;
        records.push((new_id, old_id));
    }
    let imported_images = semantic_images
        .iter()
        .map(|old_id| {
            id_map
                .get(old_id)
                .copied()
                .map(|new_id| (new_id, *old_id))
                .ok_or_else(|| invalid(format!("semantic image {old_id} has no allocated ID")))
        })
        .collect::<Result<Vec<_>>>()?;
    let imported_payloads = semantic_images
        .iter()
        .map(|old_id| item_payload(scaffold_data, &scaffold, *old_id, "semantic image"))
        .collect::<Result<Vec<_>>>()?;

    let iinf = make_iinf(source_data, scaffold_data, &source, &scaffold, &records)?;
    let iprp = make_iprp(
        source_data,
        scaffold_data,
        &source,
        &scaffold,
        &imported_images,
    )?;

    let mut output_refs = source
        .meta
        .iref
        .as_ref()
        .map_or_else(Vec::new, |iref| iref.entries.clone());
    // The Rust native writer historically emitted an Exif descriptor for the
    // primary grid only, while ImageIO-authored HDR files describe both the
    // primary and its tone-map item.  Keep the source reference authoritative,
    // but complete that established HDR descriptor contract before adding
    // semantic resources.  ImageIO otherwise treats the resulting graph as a
    // generic HEIF once auxiliary items are present, even though the source
    // Gain Map payload and graph are unchanged.
    for reference in &mut output_refs {
        if reference.kind == CDSC
            && reference.from_item_id == source_exif
            && reference.to_item_ids.contains(&source.meta.primary_item_id)
            && !reference.to_item_ids.contains(&source_tmap)
        {
            reference.to_item_ids.push(source_tmap);
        }
    }
    if let Some(iref) = scaffold.meta.iref.as_ref() {
        for reference in &iref.entries {
            if id_map.contains_key(&reference.from_item_id) {
                output_refs.push(map_semantic_reference(
                    reference,
                    scaffold.meta.primary_item_id,
                    scaffold_tmap,
                    source.meta.primary_item_id,
                    source_tmap,
                    &id_map,
                )?);
            }
        }
    }
    let maximum_reference_id = output_refs
        .iter()
        .flat_map(|reference| {
            std::iter::once(reference.from_item_id).chain(reference.to_item_ids.iter().copied())
        })
        .max()
        .unwrap_or(0);
    let iref_version = if maximum_reference_id > u32::from(u16::MAX) {
        1
    } else {
        source.meta.iref.as_ref().map_or(0, |iref| iref.version)
    };
    let iref = make_iref_box(iref_version, &output_refs)?;

    let source_idat_payload = source
        .meta
        .idat
        .as_ref()
        .map_or_else(Vec::new, |idat| source_data[idat.payload_range()].to_vec());
    let mut idat_payload = source_idat_payload;
    let mut idat_locations = BTreeMap::new();
    for old_id in &semantic_metadata {
        let payload = item_payload(scaffold_data, &scaffold, *old_id, "semantic metadata")?;
        let offset = u64::try_from(idat_payload.len())
            .map_err(|_| invalid("semantic metadata idat offset exceeds u64"))?;
        idat_payload.extend_from_slice(&payload);
        idat_locations.insert(
            *old_id,
            (offset, u64::try_from(payload.len()).unwrap_or(u64::MAX)),
        );
    }
    let idat = make_box(IDAT, &idat_payload)?;

    let source_mdat_payload = source_data[source.mdat.payload_range()].to_vec();
    let mut placeholder_locations = Vec::new();
    for entry in &source.meta.iloc.entries {
        if entry.item_id == source_exif {
            placeholder_locations.push(IlocEntry {
                item_id: source_exif,
                construction_method: 0,
                data_reference_index: 0,
                base_offset: 0,
                extents: vec![IlocExtent {
                    index: None,
                    offset: 0,
                    length: u64::try_from(scaffold_exif_payload.len())
                        .map_err(|_| invalid("semantic Exif payload length exceeds u64"))?,
                }],
            });
        } else {
            placeholder_locations.push(normalized_source_location(
                source_data,
                &source,
                entry,
                true,
            )?);
        }
    }
    for ((new_id, old_id), payload) in imported_images.iter().zip(imported_payloads.iter()) {
        placeholder_locations.push(IlocEntry {
            item_id: *new_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: u64::try_from(payload.len())
                    .map_err(|_| invalid(format!("semantic image {old_id} length exceeds u64")))?,
            }],
        });
    }
    for old_id in &semantic_metadata {
        let new_id = *id_map
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no allocated ID")))?;
        let (offset, length) = *idat_locations
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no idat location")))?;
        placeholder_locations.push(IlocEntry {
            item_id: new_id,
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
    placeholder_locations.sort_by_key(|entry| entry.item_id);
    let maximum_item_id = placeholder_locations
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let iloc_version = if maximum_item_id > u32::from(u16::MAX) {
        2
    } else {
        source.meta.iloc.version.max(1)
    };
    let placeholder_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &placeholder_locations)?;
    let preliminary_meta = make_meta(
        source_data,
        &source,
        &iinf,
        &placeholder_iloc,
        &iprp,
        &iref,
        &idat,
    )?;

    let new_mdat_box_start = source
        .top
        .boxes
        .iter()
        .take_while(|header| header.box_start != source.mdat.box_start)
        .try_fold(0usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("semantic merge mdat offset overflows"))
        })?;
    let new_mdat_data_start = u64::try_from(
        new_mdat_box_start
            .checked_add(8)
            .ok_or_else(|| invalid("semantic merge mdat data offset overflows"))?,
    )
    .map_err(|_| invalid("semantic merge mdat data offset exceeds u64"))?;

    let mut final_locations = Vec::new();
    for entry in &source.meta.iloc.entries {
        if entry.item_id == source_exif {
            continue;
        }
        final_locations.push(normalized_source_final_location(
            source_data,
            &source,
            entry,
            new_mdat_data_start,
        )?);
    }
    let mut appended_mdat = Vec::new();
    let exif_offset = new_mdat_data_start
        .checked_add(u64::try_from(source_mdat_payload.len()).unwrap_or(u64::MAX))
        .ok_or_else(|| invalid("semantic Exif offset overflows"))?;
    appended_mdat.extend_from_slice(&scaffold_exif_payload);
    final_locations.push(IlocEntry {
        item_id: source_exif,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: exif_offset,
            length: u64::try_from(scaffold_exif_payload.len())
                .map_err(|_| invalid("semantic Exif payload length exceeds u64"))?,
        }],
    });
    for ((new_id, old_id), payload) in imported_images.iter().zip(imported_payloads.iter()) {
        let offset = new_mdat_data_start
            .checked_add(u64::try_from(source_mdat_payload.len()).unwrap_or(u64::MAX))
            .and_then(|value| {
                value.checked_add(u64::try_from(appended_mdat.len()).unwrap_or(u64::MAX))
            })
            .ok_or_else(|| invalid(format!("semantic image {old_id} offset overflows")))?;
        appended_mdat.extend_from_slice(payload);
        final_locations.push(IlocEntry {
            item_id: *new_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length: u64::try_from(payload.len())
                    .map_err(|_| invalid(format!("semantic image {old_id} length exceeds u64")))?,
            }],
        });
    }
    for old_id in &semantic_metadata {
        let new_id = *id_map
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no allocated ID")))?;
        let (offset, length) = *idat_locations
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no idat location")))?;
        final_locations.push(IlocEntry {
            item_id: new_id,
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
    final_locations.sort_by_key(|entry| entry.item_id);
    let final_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &final_locations)?;
    let final_meta = make_meta(
        source_data,
        &source,
        &iinf,
        &final_iloc,
        &iprp,
        &iref,
        &idat,
    )?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid("semantic merge iloc rewrite changed meta size"));
    }

    let mut final_mdat_payload = source_mdat_payload.clone();
    final_mdat_payload.extend_from_slice(&appended_mdat);
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;
    let mut output = Vec::new();
    for header in &source.top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&final_mdat);
        } else {
            output.extend_from_slice(raw_box(source_data, header, "source top-level")?);
        }
    }
    output.extend_from_slice(
        source_data
            .get(source.top.trailing_range.clone())
            .ok_or_else(|| invalid("source top-level trailing bytes are outside input"))?,
    );

    let output_graph = parse_graph(&output, "semantic merge output")?;
    if output_graph.meta.primary_item_id != source.meta.primary_item_id {
        return Err(invalid("semantic merge changed the primary item"));
    }
    if output_graph.mdat.payload_range().len() < source_mdat_payload.len()
        || output[output_graph.mdat.payload_range().start
            ..output_graph.mdat.payload_range().start + source_mdat_payload.len()]
            != source_mdat_payload
    {
        return Err(invalid("semantic merge changed the source mdat prefix"));
    }
    for new_id in id_map.values().copied() {
        let _ = item_payload(&output, &output_graph, new_id, "semantic merge output")?;
    }
    Ok(output)
}

fn required_property_index(
    meta: &ParsedMeta,
    item_id: u32,
    kind: FourCC,
    context: &str,
) -> Result<u16> {
    let matches = associated_properties(meta, item_id)?
        .into_iter()
        .filter(|association| {
            meta.properties.iter().any(|property| {
                property.index == u32::from(association.property_index) && property.kind == kind
            })
        })
        .map(|association| association.property_index)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(invalid(format!(
            "{context} item {item_id} has no {kind} property"
        ))),
        _ => Err(invalid(format!(
            "{context} item {item_id} has multiple {kind} properties"
        ))),
    }
}

fn uniform_property(
    data: &[u8],
    meta: &ParsedMeta,
    item_ids: &[u32],
    kind: FourCC,
    context: &str,
) -> Result<(u16, Vec<u8>)> {
    let mut result: Option<(u16, Vec<u8>)> = None;
    for item_id in item_ids {
        let index = required_property_index(meta, *item_id, kind, context)?;
        let property = meta
            .properties
            .iter()
            .find(|property| property.index == u32::from(index))
            .ok_or_else(|| invalid(format!("{context} property {index} is missing")))?;
        let raw = raw_property(data, property)?.to_vec();
        if let Some((expected_index, expected_raw)) = &result {
            if *expected_index != index || *expected_raw != raw {
                return Err(invalid(format!(
                    "{context} items do not share one compatible {kind} property"
                )));
            }
        } else {
            result = Some((index, raw));
        }
    }
    result.ok_or_else(|| invalid(format!("{context} has no items")))
}

fn uniform_property_index(
    meta: &ParsedMeta,
    item_ids: &[u32],
    kind: FourCC,
    context: &str,
) -> Result<u16> {
    let mut result = None;
    for item_id in item_ids {
        let index = required_property_index(meta, *item_id, kind, context)?;
        if let Some(expected) = result {
            if expected != index {
                return Err(invalid(format!(
                    "{context} items do not share one {kind} property index"
                )));
            }
        } else {
            result = Some(index);
        }
    }
    result.ok_or_else(|| invalid(format!("{context} has no items")))
}

fn dimg_targets(meta: &ParsedMeta, from_item_id: u32, context: &str) -> Result<Vec<u32>> {
    let references = meta
        .iref
        .as_ref()
        .ok_or_else(|| invalid(format!("{context} has no iref graph")))?
        .entries
        .iter()
        .filter(|reference| reference.kind == DIMG && reference.from_item_id == from_item_id)
        .collect::<Vec<_>>();
    match references.as_slice() {
        [reference] if !reference.to_item_ids.is_empty() => Ok(reference.to_item_ids.clone()),
        [] => Err(invalid(format!(
            "{context} item {from_item_id} has no dimg reference"
        ))),
        [_] => Err(invalid(format!(
            "{context} item {from_item_id} has an empty dimg reference"
        ))),
        _ => Err(invalid(format!(
            "{context} item {from_item_id} has multiple dimg references"
        ))),
    }
}

fn item_is_type(meta: &ParsedMeta, item_id: u32, item_type: FourCC) -> bool {
    meta.iinf
        .entries
        .iter()
        .any(|item| item.item_id == item_id && item.item_type == Some(item_type))
}

fn primary_tiles(meta: &ParsedMeta, context: &str) -> Result<Vec<u32>> {
    let targets = dimg_targets(meta, meta.primary_item_id, context)?;
    if targets
        .iter()
        .all(|item_id| item_is_type(meta, *item_id, HVC1))
    {
        Ok(targets)
    } else {
        Err(invalid(format!(
            "{context} primary dimg graph contains a non-hvc1 item"
        )))
    }
}

fn gain_tiles(meta: &ParsedMeta, context: &str) -> Result<Vec<u32>> {
    let tmap_id = find_tmap(meta, context)?;
    let tmap_targets = dimg_targets(meta, tmap_id, context)?;
    let gain_grids = tmap_targets
        .into_iter()
        .filter(|item_id| *item_id != meta.primary_item_id && item_is_type(meta, *item_id, GRID))
        .collect::<Vec<_>>();
    let gain_grid = match gain_grids.as_slice() {
        [only] => *only,
        [] => return Err(invalid(format!("{context} tmap has no gain grid"))),
        _ => return Err(invalid(format!("{context} tmap has multiple gain grids"))),
    };
    let targets = dimg_targets(meta, gain_grid, context)?;
    if targets
        .iter()
        .all(|item_id| item_is_type(meta, *item_id, HVC1))
    {
        Ok(targets)
    } else {
        Err(invalid(format!(
            "{context} gain dimg graph contains a non-hvc1 item"
        )))
    }
}

fn rebuild_iprp_properties(
    data: &[u8],
    graph: &Graph,
    replacements: &BTreeMap<u32, Vec<u8>>,
) -> Result<Vec<u8>> {
    let iprp = child(&graph.meta_children, IPRP, "semantic scaffold meta")?;
    let children = parse_boxes(data, iprp.payload_range())?;
    let mut payload = Vec::new();
    let mut saw_ipco = false;
    for header in &children {
        if header.kind != IPCO {
            payload.extend_from_slice(raw_box(data, header, "semantic scaffold iprp child")?);
            continue;
        }
        if saw_ipco {
            return Err(invalid(
                "semantic scaffold iprp contains multiple ipco boxes",
            ));
        }
        saw_ipco = true;
        let properties = parse_boxes(data, header.payload_range())?;
        let mut ipco_payload = Vec::new();
        for (position, property) in properties.iter().enumerate() {
            let index = u32::try_from(position + 1)
                .map_err(|_| invalid("semantic scaffold property index overflows"))?;
            if let Some(replacement) = replacements.get(&index) {
                ipco_payload.extend_from_slice(replacement);
            } else {
                ipco_payload.extend_from_slice(raw_box(
                    data,
                    property,
                    "semantic scaffold ipco property",
                )?);
            }
        }
        payload.extend_from_slice(&make_box(IPCO, &ipco_payload)?);
    }
    if !saw_ipco {
        return Err(invalid("semantic scaffold iprp has no ipco box"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn rebuild_meta_children(data: &[u8], graph: &Graph, iloc: &[u8], iprp: &[u8]) -> Result<Vec<u8>> {
    let full_header_end = graph
        .meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("semantic scaffold meta full-box header overflows"))?;
    let full_header = data
        .get(graph.meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("semantic scaffold meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iloc = false;
    let mut saw_iprp = false;
    for header in &graph.meta_children {
        match header.kind {
            ILOC if !saw_iloc => {
                saw_iloc = true;
                payload.extend_from_slice(iloc);
            }
            ILOC => {
                return Err(invalid(
                    "semantic scaffold meta contains multiple iloc boxes",
                ))
            }
            IPRP if !saw_iprp => {
                saw_iprp = true;
                payload.extend_from_slice(iprp);
            }
            IPRP => {
                return Err(invalid(
                    "semantic scaffold meta contains multiple iprp boxes",
                ))
            }
            _ => payload.extend_from_slice(raw_box(data, header, "semantic scaffold meta child")?),
        }
    }
    if !saw_iloc || !saw_iprp {
        return Err(invalid("semantic scaffold meta is missing iloc or iprp"));
    }
    Ok(make_box(META, &payload)?)
}

#[derive(Debug)]
struct PayloadReplacement {
    item_id: u32,
    old_start: u64,
    old_end: u64,
    payload: Vec<u8>,
}

fn payload_replacements(
    source_data: &[u8],
    source: &Graph,
    scaffold: &Graph,
    source_item_ids: &[u32],
    scaffold_item_ids: &[u32],
    context: &str,
) -> Result<Vec<PayloadReplacement>> {
    if source_item_ids.len() != scaffold_item_ids.len() || source_item_ids.is_empty() {
        return Err(invalid(format!(
            "{context} source/scaffold tile counts differ"
        )));
    }
    let mdat_start = u64::try_from(scaffold.mdat.data_start)
        .map_err(|_| invalid("semantic scaffold mdat offset exceeds u64"))?;
    let mdat_end = u64::try_from(scaffold.mdat.data_end)
        .map_err(|_| invalid("semantic scaffold mdat end exceeds u64"))?;
    let mut replacements = Vec::with_capacity(source_item_ids.len());
    for (source_id, scaffold_id) in source_item_ids.iter().zip(scaffold_item_ids) {
        let source_payload = item_payload(source_data, source, *source_id, context)?;
        let entry = item_location(&scaffold.meta, *scaffold_id, context)?;
        if entry.construction_method != 0 || entry.data_reference_index != 0 {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} is not a local file item"
            )));
        }
        if entry.extents.len() != 1 {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} does not have one extent"
            )));
        }
        let extent = &entry.extents[0];
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} has a non-zero extent index"
            )));
        }
        let old_start = entry.resolved_extent_offset(extent)?;
        let old_end = old_start.checked_add(extent.length).ok_or_else(|| {
            invalid(format!(
                "{context} scaffold item {scaffold_id} extent overflows"
            ))
        })?;
        if old_start < mdat_start || old_end > mdat_end {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} extent is outside mdat"
            )));
        }
        replacements.push(PayloadReplacement {
            item_id: *scaffold_id,
            old_start,
            old_end,
            payload: source_payload,
        });
    }
    replacements.sort_by_key(|replacement| replacement.old_start);
    for pair in replacements.windows(2) {
        if pair[0].old_end > pair[1].old_start {
            return Err(invalid(format!("{context} scaffold tile extents overlap")));
        }
    }
    Ok(replacements)
}

fn apply_replacement_delta(value: u64, old_length: u64, new_length: u64) -> Result<u64> {
    if new_length >= old_length {
        value
            .checked_add(new_length - old_length)
            .ok_or_else(|| invalid("semantic scaffold mdat offset overflows"))
    } else {
        value
            .checked_sub(old_length - new_length)
            .ok_or_else(|| invalid("semantic scaffold mdat offset underflows"))
    }
}

fn translated_mdat_offset(
    old_offset: u64,
    old_mdat_start: u64,
    replacements: &[PayloadReplacement],
) -> Result<u64> {
    let relative = old_offset
        .checked_sub(old_mdat_start)
        .ok_or_else(|| invalid("semantic scaffold extent precedes mdat"))?;
    let mut translated = relative;
    for replacement in replacements {
        if replacement.old_end <= old_offset {
            translated = apply_replacement_delta(
                translated,
                replacement.old_end - replacement.old_start,
                u64::try_from(replacement.payload.len())
                    .map_err(|_| invalid("semantic replacement payload exceeds u64"))?,
            )?;
        } else if replacement.old_start < old_offset {
            return Err(invalid(
                "semantic scaffold extent begins inside a replaced tile payload",
            ));
        }
    }
    Ok(translated)
}

fn replaced_mdat_payload(
    data: &[u8],
    graph: &Graph,
    replacements: &[PayloadReplacement],
) -> Result<Vec<u8>> {
    let original = data
        .get(graph.mdat.payload_range())
        .ok_or_else(|| invalid("semantic scaffold mdat payload is outside input"))?;
    let mdat_start = u64::try_from(graph.mdat.data_start)
        .map_err(|_| invalid("semantic scaffold mdat offset exceeds u64"))?;
    let mut output = Vec::new();
    let mut cursor = 0usize;
    for replacement in replacements {
        let relative_start = usize::try_from(
            replacement
                .old_start
                .checked_sub(mdat_start)
                .ok_or_else(|| invalid("semantic replacement precedes mdat"))?,
        )
        .map_err(|_| invalid("semantic replacement offset exceeds usize"))?;
        let relative_end = usize::try_from(
            replacement
                .old_end
                .checked_sub(mdat_start)
                .ok_or_else(|| invalid("semantic replacement end precedes mdat"))?,
        )
        .map_err(|_| invalid("semantic replacement end exceeds usize"))?;
        if relative_start < cursor || relative_end > original.len() {
            return Err(invalid(
                "semantic replacement range is outside mdat payload",
            ));
        }
        output.extend_from_slice(&original[cursor..relative_start]);
        output.extend_from_slice(&replacement.payload);
        cursor = relative_end;
    }
    output.extend_from_slice(&original[cursor..]);
    Ok(output)
}

fn transplant_iloc_locations(
    scaffold: &Graph,
    replacements: &[PayloadReplacement],
    new_mdat_data_start: u64,
) -> Result<Vec<IlocEntry>> {
    let old_mdat_data_start = u64::try_from(scaffold.mdat.data_start)
        .map_err(|_| invalid("semantic scaffold mdat offset exceeds u64"))?;
    let mut entries = Vec::with_capacity(scaffold.meta.iloc.entries.len());
    for entry in &scaffold.meta.iloc.entries {
        let mut output = entry.clone();
        if entry.construction_method == 0 {
            if entry.data_reference_index != 0 {
                return Err(invalid(format!(
                    "semantic scaffold item {} uses a data reference",
                    entry.item_id
                )));
            }
            output.base_offset = 0;
            for extent in &mut output.extents {
                if extent.index.unwrap_or(0) != 0 {
                    return Err(invalid(format!(
                        "semantic scaffold item {} uses a non-zero extent index",
                        entry.item_id
                    )));
                }
                let old_start = entry
                    .resolved_extent_offset(extent)
                    .map_err(|_| invalid("semantic scaffold extent offset overflows"))?;
                let new_relative =
                    translated_mdat_offset(old_start, old_mdat_data_start, replacements)?;
                extent.offset = new_mdat_data_start
                    .checked_add(new_relative)
                    .ok_or_else(|| invalid("semantic transplanted extent offset overflows"))?;
                if let Some(replacement) = replacements
                    .iter()
                    .find(|replacement| replacement.item_id == entry.item_id)
                {
                    let old_end = old_start
                        .checked_add(extent.length)
                        .ok_or_else(|| invalid("semantic scaffold tile extent overflows"))?;
                    if old_start != replacement.old_start || old_end != replacement.old_end {
                        return Err(invalid(format!(
                            "semantic scaffold item {} has an unexpected replacement extent",
                            entry.item_id
                        )));
                    }
                    extent.length = u64::try_from(replacement.payload.len())
                        .map_err(|_| invalid("semantic replacement payload exceeds u64"))?;
                }
            }
        }
        entries.push(output);
    }
    Ok(entries)
}

/// Build the ImageIO-compatible semantic scaffold through the platform
/// primitive, then transplant Rust-owned primary and Gain Map tile payloads
/// and their matching codec configurations into that graph.  The scaffold's
/// item/property/reference model remains authoritative for Apple semantic
/// resources; Rust remains authoritative for the compressed HDR payloads.
pub fn transplant_apple_semantic_auxiliary_heif(
    source_data: &[u8],
    scaffold_data: &[u8],
    expected_semantic_roles: usize,
) -> Result<Vec<u8>> {
    if expected_semantic_roles == 0 {
        return Err(invalid("semantic transplant requires at least one role"));
    }
    let source = parse_graph(source_data, "source HDR")?;
    let scaffold = parse_graph(scaffold_data, "semantic scaffold")?;
    let (_, semantic_images, semantic_metadata) = semantic_items(&scaffold)?;
    if semantic_images.len() != expected_semantic_roles {
        return Err(invalid(format!(
            "semantic scaffold contains {} roles; expected {expected_semantic_roles}",
            semantic_images.len()
        )));
    }

    let source_primary_tiles = primary_tiles(&source.meta, "source HDR")?;
    let scaffold_primary_tiles = primary_tiles(&scaffold.meta, "semantic scaffold")?;
    let source_gain_tiles = gain_tiles(&source.meta, "source HDR")?;
    let scaffold_gain_tiles = gain_tiles(&scaffold.meta, "semantic scaffold")?;
    if source_primary_tiles.len() != scaffold_primary_tiles.len()
        || source_gain_tiles.len() != scaffold_gain_tiles.len()
    {
        return Err(invalid(
            "source and semantic scaffold tile graph counts differ",
        ));
    }

    let mut codec_replacements = BTreeMap::new();
    for (source_ids, scaffold_ids, context) in [
        (
            &source_primary_tiles,
            &scaffold_primary_tiles,
            "primary tiles",
        ),
        (&source_gain_tiles, &scaffold_gain_tiles, "Gain Map tiles"),
    ] {
        let (_, source_raw) =
            uniform_property(source_data, &source.meta, source_ids, HVCC, context)?;
        let scaffold_index = uniform_property_index(&scaffold.meta, scaffold_ids, HVCC, context)?;
        let property_index = u32::from(scaffold_index);
        if let Some(existing) = codec_replacements.get(&property_index) {
            if *existing != source_raw {
                return Err(invalid(format!(
                    "semantic scaffold reuses hvcC property {} for incompatible codecs",
                    scaffold_index
                )));
            }
        } else {
            codec_replacements.insert(property_index, source_raw);
        }
    }
    let iprp = rebuild_iprp_properties(scaffold_data, &scaffold, &codec_replacements)?;
    let preliminary_iloc = make_iloc_box(
        scaffold.meta.iloc.version,
        scaffold.meta.iloc.offset_size,
        scaffold.meta.iloc.length_size,
        scaffold.meta.iloc.base_offset_size,
        scaffold.meta.iloc.index_size,
        &scaffold.meta.iloc.entries,
    )?;
    let preliminary_meta =
        rebuild_meta_children(scaffold_data, &scaffold, &preliminary_iloc, &iprp)?;

    let new_mdat_box_start = scaffold
        .top
        .boxes
        .iter()
        .take_while(|header| header.box_start != scaffold.mdat.box_start)
        .try_fold(0usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("semantic transplanted mdat offset overflows"))
        })?;
    let new_mdat_data_start = u64::try_from(
        new_mdat_box_start
            .checked_add(8)
            .ok_or_else(|| invalid("semantic transplanted mdat data offset overflows"))?,
    )
    .map_err(|_| invalid("semantic transplanted mdat data offset exceeds u64"))?;

    let mut replacements = payload_replacements(
        source_data,
        &source,
        &scaffold,
        &source_primary_tiles,
        &scaffold_primary_tiles,
        "primary tiles",
    )?;
    replacements.extend(payload_replacements(
        source_data,
        &source,
        &scaffold,
        &source_gain_tiles,
        &scaffold_gain_tiles,
        "Gain Map tiles",
    )?);
    replacements.sort_by_key(|replacement| replacement.old_start);
    for pair in replacements.windows(2) {
        if pair[0].old_end > pair[1].old_start {
            return Err(invalid("semantic scaffold replacement extents overlap"));
        }
    }

    let final_iloc_entries =
        transplant_iloc_locations(&scaffold, &replacements, new_mdat_data_start)?;
    let final_iloc = make_iloc_box(
        scaffold.meta.iloc.version,
        scaffold.meta.iloc.offset_size,
        scaffold.meta.iloc.length_size,
        scaffold.meta.iloc.base_offset_size,
        scaffold.meta.iloc.index_size,
        &final_iloc_entries,
    )?;
    let final_meta = rebuild_meta_children(scaffold_data, &scaffold, &final_iloc, &iprp)?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "semantic transplant iloc rewrite changed meta size",
        ));
    }
    let final_mdat_payload = replaced_mdat_payload(scaffold_data, &scaffold, &replacements)?;
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;

    let mut output = Vec::new();
    for header in &scaffold.top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&final_mdat);
        } else {
            output.extend_from_slice(raw_box(
                scaffold_data,
                header,
                "semantic scaffold top-level",
            )?);
        }
    }
    output.extend_from_slice(
        scaffold_data
            .get(scaffold.top.trailing_range.clone())
            .ok_or_else(|| invalid("semantic scaffold trailing bytes are outside input"))?,
    );

    let output_graph = parse_graph(&output, "semantic transplant output")?;
    if output_graph.meta.primary_item_id != scaffold.meta.primary_item_id {
        return Err(invalid("semantic transplant changed the primary item"));
    }
    for (source_id, scaffold_id) in source_primary_tiles
        .iter()
        .chain(source_gain_tiles.iter())
        .zip(
            scaffold_primary_tiles
                .iter()
                .chain(scaffold_gain_tiles.iter()),
        )
    {
        let source_payload = item_payload(source_data, &source, *source_id, "source tile")?;
        let output_payload = item_payload(&output, &output_graph, *scaffold_id, "output tile")?;
        if source_payload != output_payload {
            return Err(invalid(format!(
                "semantic transplant changed Rust tile payload {source_id} -> {scaffold_id}"
            )));
        }
    }
    for item_id in semantic_images.iter().chain(semantic_metadata.iter()) {
        let expected = item_payload(scaffold_data, &scaffold, *item_id, "semantic scaffold item")?;
        let actual = item_payload(&output, &output_graph, *item_id, "semantic output item")?;
        if expected != actual {
            return Err(invalid(format!(
                "semantic transplant changed scaffold item payload {item_id}"
            )));
        }
    }
    for (index, raw) in &codec_replacements {
        let property = output_graph
            .meta
            .properties
            .iter()
            .find(|property| property.index == *index)
            .ok_or_else(|| invalid(format!("semantic output hvcC property {index} is missing")))?;
        if raw_property(&output, property)? != raw {
            return Err(invalid(format!(
                "semantic output hvcC property {index} differs from Rust codec"
            )));
        }
    }
    Ok(output)
}
